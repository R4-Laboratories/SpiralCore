//! Cooperating object service: verified shared state over the authenticated gateway.
use crate::{
    measurement::{cpu_us, invalid, Result},
    transport::{Identity, Listener, Peer},
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use spiralcore_gateway_core::object::{ObjectMode, ObjectSelection, ObjectSender};
use spiralcore_transfer::{
    self as transfer,
    endpoint::{Authority, ColdGraphPolicy, Context, Endpoint, Limits},
    protocol::Frame,
    shared_runtime::Recipe,
    shared_transfer::PreparedPrediction,
    Id,
};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::Read,
    net::SocketAddr,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub label: String,
    pub action: String,
    pub path: Option<PathBuf>,
    pub warm_probe: bool,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub label: String,
    pub mode: String,
    pub schema_path: Option<PathBuf>,
    pub checkpoint_path: Option<PathBuf>,
    pub recipe_path: Option<PathBuf>,
    pub operations: Vec<Operation>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub role: String,
    pub bind: SocketAddr,
    pub peer: SocketAddr,
    pub server_name: String,
    pub certificate: PathBuf,
    pub private_key: PathBuf,
    pub trust_anchor: PathBuf,
    pub peer_certificate: PathBuf,
    pub scope: String,
    pub policy_epoch: u64,
    pub store_dir: PathBuf,
    pub stats_path: PathBuf,
    pub ready_path: PathBuf,
    pub duration_secs: u64,
    pub groups: Vec<Group>,
    pub verify_files: Vec<PathBuf>,
}
fn label(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if !["listen", "connect"].contains(&self.role.as_str())
            || !label(&self.scope)
            || !label(&self.server_name)
            || !(1..=3600).contains(&self.duration_secs)
            || self.stats_path == self.ready_path
            || self.groups.len() > 8
            || self.verify_files.len() > 64
            || self.store_dir.as_os_str().is_empty()
            || (self.role == "listen" && !self.groups.is_empty())
            || (self.role == "connect" && (self.groups.is_empty() || !self.verify_files.is_empty()))
        {
            return Err(invalid("invalid object gateway configuration").into());
        }
        let mut names = BTreeSet::new();
        for g in &self.groups {
            if !label(&g.label)
                || !names.insert(&g.label)
                || g.operations.is_empty()
                || g.operations.len() > 32
                || !["raw", "whole-zstd", "warm", "runtime"].contains(&g.mode.as_str())
                || (g.mode == "warm") != g.schema_path.is_some()
                || (g.mode == "runtime") != (g.checkpoint_path.is_some() && g.recipe_path.is_some())
                || (g.mode != "runtime" && (g.checkpoint_path.is_some() || g.recipe_path.is_some()))
            {
                return Err(invalid("invalid object group").into());
            }
            let mut labels = BTreeSet::new();
            for op in &g.operations {
                if !label(&op.label)
                    || !labels.insert(&op.label)
                    || !["send", "seed", "schema", "checkpoint"].contains(&op.action.as_str())
                    || ["send", "seed"].contains(&op.action.as_str()) != op.path.is_some()
                    || (op.action == "seed" && g.mode != "warm")
                    || (op.action == "schema" && g.mode != "warm")
                    || (op.action == "checkpoint" && g.mode != "runtime")
                {
                    return Err(invalid("invalid object operation").into());
                }
            }
        }
        Ok(())
    }
}
pub fn read_bounded(path: &Path, cap: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > cap {
        return Err(invalid("object input exceeds bound").into());
    }
    Ok(bytes)
}

fn resolved_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut ancestor = absolute.as_path();
    let mut suffix = Vec::new();
    loop {
        match ancestor.canonicalize() {
            Ok(mut resolved) => {
                for part in suffix.into_iter().rev() {
                    resolved.push(part);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                suffix.push(
                    ancestor
                        .file_name()
                        .ok_or_else(|| invalid("invalid output/input path"))?
                        .to_os_string(),
                );
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| invalid("invalid output/input parent"))?;
            }
            Err(error) => return Err(error.into()),
        }
    }
}
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
fn absent(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => Err(invalid("object output or reservation already exists").into()),
    }
}
fn same_file(path: &Path, file: &File) -> bool {
    let Ok(path_meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    let Ok(file_meta) = file.metadata() else {
        return false;
    };
    path_meta.is_file() && path_meta.dev() == file_meta.dev() && path_meta.ino() == file_meta.ino()
}

/// A fresh output lease. First publication never replaces an existing path;
/// later atomic updates replace only the inode published by this owner.
struct OwnedOutput {
    path: PathBuf,
    temporary: PathBuf,
    lease: PathBuf,
    lease_file: File,
    published: Option<File>,
}
impl OwnedOutput {
    fn reserve(path: PathBuf) -> Result<Self> {
        let lease = sibling(&path, ".sc-object.lock");
        let lease_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&lease)?;
        Ok(Self {
            temporary: sibling(&path, ".sc-object.tmp"),
            path,
            lease,
            lease_file,
            published: None,
        })
    }
    fn write(&mut self, value: &impl Serialize) -> Result<()> {
        if !same_file(&self.lease, &self.lease_file) {
            return Err(invalid("object output ownership changed").into());
        }
        let mut staged = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&self.temporary)?;
        let result = (|| -> Result<()> {
            serde_json::to_writer_pretty(&mut staged, value)?;
            staged.sync_all()?;
            let held = staged.try_clone()?;
            if let Some(current) = &self.published {
                if !same_file(&self.path, current) {
                    return Err(invalid("object output was replaced").into());
                }
                std::fs::rename(&self.temporary, &self.path)?;
            } else {
                // Unlike rename, hard_link fails if any file/symlink already exists.
                std::fs::hard_link(&self.temporary, &self.path)?;
            }
            self.published = Some(held);
            File::open(
                self.path
                    .parent()
                    .ok_or_else(|| invalid("output parent missing"))?,
            )?
            .sync_all()?;
            Ok(())
        })();
        if same_file(&self.temporary, &staged) {
            let _ = std::fs::remove_file(&self.temporary);
        }
        result
    }
}
impl Drop for OwnedOutput {
    fn drop(&mut self) {
        if same_file(&self.lease, &self.lease_file) {
            let _ = std::fs::remove_file(&self.lease);
        }
    }
}
type StatsOutput = Arc<Mutex<OwnedOutput>>;
fn reserve_outputs(config: &Config) -> Result<(StatsOutput, OwnedOutput)> {
    let output = |path: &Path| -> Result<PathBuf> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let path = parent.canonicalize()?.join(
            path.file_name()
                .ok_or_else(|| invalid("output filename missing"))?,
        );
        absent(&path)?;
        Ok(path)
    };
    let stats = output(&config.stats_path)?;
    let ready = output(&config.ready_path)?;
    let mut protected = vec![
        &config.certificate,
        &config.private_key,
        &config.trust_anchor,
        &config.peer_certificate,
    ];
    protected.extend(config.verify_files.iter());
    for group in &config.groups {
        protected.extend(
            group
                .schema_path
                .iter()
                .chain(group.checkpoint_path.iter())
                .chain(group.recipe_path.iter()),
        );
        protected.extend(
            group
                .operations
                .iter()
                .filter_map(|operation| operation.path.as_ref()),
        );
    }
    let protected = protected
        .into_iter()
        .map(|path| resolved_path(path))
        .collect::<Result<Vec<_>>>()?;
    let store = resolved_path(&config.store_dir)?;
    let mut paths = BTreeSet::new();
    for path in [&stats, &ready] {
        for candidate in [
            path.clone(),
            sibling(path, ".sc-object.lock"),
            sibling(path, ".sc-object.tmp"),
        ] {
            if !paths.insert(candidate.clone())
                || candidate.starts_with(&store)
                || store.starts_with(&candidate)
                || protected
                    .iter()
                    .any(|input| input == &candidate || input.starts_with(&candidate))
            {
                return Err(
                    invalid("object output overlaps input, store or another output").into(),
                );
            }
            absent(&candidate)?;
        }
    }
    // Hold both leases before any readiness or error report may be written.
    let stats = OwnedOutput::reserve(stats)?;
    let ready = OwnedOutput::reserve(ready)?;
    Ok((Arc::new(Mutex::new(stats)), ready))
}
fn write_stats(output: &StatsOutput, value: &impl Serialize) -> Result<()> {
    output
        .lock()
        .map_err(|_| invalid("object stats ownership lock poisoned"))?
        .write(value)
}
#[derive(Clone)]
struct ScopedAuthority(Context);
impl Authority for ScopedAuthority {
    fn authorize(&self, c: &Context) -> transfer::Result<()> {
        if c == &self.0 {
            Ok(())
        } else {
            Err(transfer::Error::Unauthorized)
        }
    }
}
fn identity_context(c: &Config) -> Result<(Identity, Context)> {
    let local = read_bounded(&c.certificate, 16 * 1024)?;
    let remote = read_bounded(&c.peer_certificate, 16 * 1024)?;
    let local_id: [u8; 32] = Sha256::digest(&local).into();
    let remote_id: [u8; 32] = Sha256::digest(&remote).into();
    let (sender, recipient) = if c.role == "connect" {
        (local_id, remote_id)
    } else {
        (remote_id, local_id)
    };
    let context = Context {
        tenant: c.scope.clone(),
        domain: "spiralcore.gateway.objects.v1".into(),
        sender: transfer::hex(&sender),
        recipient: transfer::hex(&recipient),
        mailbox: "object-channel".into(),
        capability: "pinned-mtls".into(),
        authorization_scope: c.scope.clone(),
        policy_epoch: c.policy_epoch,
        execution_epoch: 0,
    };
    context.namespace()?;
    Ok((
        Identity {
            certificate_chain: vec![CertificateDer::from(local)],
            private_key: PrivateKeyDer::try_from(read_bounded(&c.private_key, 64 * 1024)?)
                .map_err(invalid)?,
            trusted_roots: vec![CertificateDer::from(read_bounded(
                &c.trust_anchor,
                16 * 1024,
            )?)],
            peer_leaf_sha256: remote_id,
        },
        context,
    ))
}
fn selection_value(selection: &ObjectSelection) -> Value {
    match selection {
        ObjectSelection::Fixed => json!({"policy":"fixed-profile","candidate_costs":null}),
        ObjectSelection::Warm(s) => {
            json!({"policy":"rolling-warm","accounting":"existing HTTP-body cost model, not QUIC wire bytes",
            "selected":format!("{:?}",s.selected_mode),"needs_schema_seed":s.needs_schema_seed,
            "candidates":s.candidates.iter().map(|c|json!({"mode":format!("{:?}",c.mode),"target_http_body_bytes":c.target_http_body_bytes,"setup_http_body_bytes":c.setup_http_body_bytes,"total_http_body_bytes":c.total_http_body_bytes})).collect::<Vec<_>>(),
            "pruned":s.pruned.iter().map(|c|json!({"mode":format!("{:?}",c.mode),"target_http_body_lower_bound":c.target_http_body_lower_bound,"total_http_body_lower_bound":c.total_http_body_lower_bound})).collect::<Vec<_>>()})
        }
        ObjectSelection::Runtime(s) => {
            json!({"policy":"fixed-runtime-checkpoint","accounting":"existing HTTP-body cost model, not QUIC wire bytes",
            "selected":format!("{:?}",s.selected),"expected_http_body_bytes":s.expected_http_body_bytes,"exact_repeat_probe":s.exact_repeat_probe,
            "candidates":s.candidates.iter().map(|c|json!({"mode":format!("{:?}",c.mode),"http_body_bytes":c.http_body_bytes})).collect::<Vec<_>>(),
            "pruned":s.pruned.iter().map(|c|json!({"mode":format!("{:?}",c.mode),"http_body_lower_bound":c.http_body_lower_bound})).collect::<Vec<_>>()})
        }
    }
}

/// Process the configured peer with one bounded blocking codec worker. QUIC I/O
/// stays on the Tokio runtime; synchronous transfer callbacks bridge via Handle.
pub async fn run(config: Config) -> Result<()> {
    config.validate()?;
    let (stats, ready) = reserve_outputs(&config)?;
    let result = run_inner(config, Arc::clone(&stats), ready).await;
    if let Err(error) = &result {
        let _ = write_stats(
            &stats,
            &json!({"status":"error","error":error.to_string(),"qualified_complete":false}),
        );
    }
    result
}
async fn run_inner(config: Config, stats: StatsOutput, mut ready: OwnedOutput) -> Result<()> {
    let whole_start = Instant::now();
    let whole_cpu = cpu_us()?;
    let (identity, context) = identity_context(&config)?;
    let end = tokio::time::Instant::now() + std::time::Duration::from_secs(config.duration_secs);
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let signal = tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        stop.cancel();
    });
    let result=async {
        let peer=if config.role=="listen" {
            let listener=Listener::bind_objects(config.bind,identity)?;
            ready.write(&json!({"listening":true,"address":listener.local_addr()?.to_string()}))?;
            tokio::select! { biased;
                _=cancel.cancelled()=>return Err(invalid("object handshake cancelled").into()),
                result=listener.accept(end)=>result?,
            }
        } else { tokio::select! { biased;
            _=cancel.cancelled()=>return Err(invalid("object handshake cancelled").into()),
            result=Peer::connect_objects(config.bind,config.peer,&config.server_name,identity,end)=>result?,
        }};
        let mut channel=if config.role=="listen" {peer.accept_object_channel(end,&cancel).await?} else {peer.open_object_channel(end,&cancel).await?};
        // Compare authenticated local configuration before opening any store or
        // publishing data. This equality check grants no remote-selected scope.
        let binding=Frame {kind:6,transfer:context.namespace()?,seq:0,body:b"SCGNSP01".to_vec()};
        let expected=std::slice::from_ref(&binding);
        if config.role=="listen" {
            let offered=channel.recv_batch(end,&cancel).await?.ok_or_else(||invalid("missing object namespace binding"))?;
            if offered.as_slice()!=expected {return Err(invalid("object namespace binding mismatch").into());}
            channel.send_batch(expected,end,&cancel).await?;
        } else {
            let echoed=channel.exchange(expected,end,&cancel).await?;
            if echoed.as_slice()!=expected {return Err(invalid("object namespace binding mismatch").into());}
        }
        let setup_wall_us=whole_start.elapsed().as_micros(); let setup_cpu_us=cpu_us()?-whole_cpu;
        ready.write(&json!({"ready":true,"profile":"spiralcore-object/1","setup_wall_us":setup_wall_us,"setup_cpu_us":setup_cpu_us}))?;
        let handle=tokio::runtime::Handle::current();
        tokio::task::spawn_blocking(move || ->Result<()> {
            let mut summary=if config.role=="connect" {
                let mut group_rows=Vec::new(); let mut operation_rows=Vec::new(); let mut ordinal=0u64;
                for group in &config.groups {
                    let prep=Instant::now(); let prep_cpu=cpu_us()?;
                    let mut auxiliary_len=0;
                    let mut sender=match group.mode.as_str() {
                        "raw"=>ObjectSender::new(ObjectMode::Raw,&context,None)?,
                        "whole-zstd"=>ObjectSender::new(ObjectMode::Whole,&context,None)?,
                        "warm"=>{
                            let schema=read_bounded(group.schema_path.as_ref().ok_or_else(||invalid("missing schema"))?,transfer::client::MAX_BATCHED_OBJECT)?;
                            auxiliary_len=schema.len();
                            ObjectSender::new(ObjectMode::Warm,&context,Some(&schema))?
                        },
                        "runtime"=>{
                            let checkpoint=read_bounded(group.checkpoint_path.as_ref().ok_or_else(||invalid("missing checkpoint"))?,transfer::shared_runtime::MAX_CHECKPOINT)?;
                            auxiliary_len=checkpoint.len();
                            let recipe=Recipe::decode(&read_bounded(group.recipe_path.as_ref().ok_or_else(||invalid("missing recipe"))?,1024)?)?;
                            ObjectSender::with_runtime(&context,PreparedPrediction::new(&checkpoint,&recipe)?)?
                        },
                        _=>return Err(invalid("unknown profile").into()),
                    };
                    group_rows.push((group.label.clone(),group.mode.clone(),prep.elapsed().as_micros(),cpu_us()?-prep_cpu,sender.retained_charge()));
                    for op in &group.operations {
                        ordinal+=1;
                        let mut id=[0;32]; id[..8].copy_from_slice(&ordinal.to_le_bytes());
                        // Transfer IDs are local to this authenticated connection's store scope.
                        id[8..].copy_from_slice(&context.namespace()?[..24]);
                        let before=channel.counters(); let start=Instant::now(); let cpu=cpu_us()?;
                        let bytes=op.path.as_ref().map(|p|read_bounded(p,transfer::client::MAX_BATCHED_OBJECT)).transpose()?;
                        let len=bytes.as_ref().map_or(auxiliary_len,Vec::len);
                        let expected=bytes.as_ref().map(|b|transfer::object_id(b));
                        let mut failure=None;
                        let mut published_on_begin=false;
                        let mut exchange=|frames:&[Frame]| match handle.block_on(channel.exchange(frames,end,&cancel)) {
                            Ok(reply)=>{
                                published_on_begin|=frames.iter().zip(&reply).any(|(request,response)|request.kind==1 && response.kind==103);
                                Ok(reply)
                            },Err(error)=>{failure=Some(error.to_string());Err(transfer::Error::Storage)}
                        };
                        let outcome=match op.action.as_str() {
                            "schema"=>sender.seed_schema(&context,id,&mut exchange).map(|s|(s,ObjectSelection::Fixed)),
                            "checkpoint"=>sender.seed_checkpoint(&context,id,&mut exchange).map(|s|(s,ObjectSelection::Fixed)),
                            "seed"=>sender.seed(&context,bytes.ok_or(transfer::Error::Malformed)?,id,&mut exchange).map(|s|(s,ObjectSelection::Fixed)),
                            "send"=>sender.send_with_receipt(&context,bytes.ok_or(transfer::Error::Malformed)?,id,op.warm_probe,&mut exchange).map(|s|(s.transfer,s.selection)),
                            _=>Err(transfer::Error::Malformed),
                        };
                        let (stats,selection)=outcome.map_err(|e|invalid(&failure.unwrap_or_else(||e.to_string())))?;
                        if expected.is_some_and(|e|e!=stats.object) {return Err(invalid("published identity mismatch").into());}
                        let wall_us=start.elapsed().as_micros(); let cpu_us=cpu_us()?-cpu; let after=channel.counters();
                        // Only expose the observed path after the existing client
                        // has checked reply correlation and the publication ID.
                        operation_rows.push((group.label.clone(),op.label.clone(),op.action.clone(),id,len,stats,selection,published_on_begin,wall_us,cpu_us,before,after));
                    }
                }
                let finish=Instant::now(); let finish_cpu=cpu_us()?;
                handle.block_on(channel.finish(end,&cancel))?;
                let finish_wall_us=finish.elapsed().as_micros(); let finish_cpu_us=cpu_us()?-finish_cpu;
                // Defer report construction until every measured exchange completes.
                json!({"groups":group_rows.into_iter().map(|(label,mode,wall,cpu,charge)|json!({"label":label,"mode":mode,"wall_us":wall,"cpu_us":cpu,"retained_charge":charge})).collect::<Vec<_>>(),
                    "operations":operation_rows.into_iter().map(|(group,label,action,id,len,s,selection,cache_hit,wall,cpu,before,after)|json!({"group":group,"label":label,"action":action,"transfer_id":transfer::hex(&id),"input_bytes":len,"selected_mode":format!("{:?}",s.mode),"publication_path":if cache_hit {"verified-cache-hit"}else{"verified-transfer"},"object_id":transfer::hex(&s.object),"request_frame_bytes":s.request_bytes,"response_frame_bytes":s.response_bytes,"data_bytes":s.data_bytes,"requests":s.requests,"wall_us":wall,"cpu_us":cpu,"channel_before":before,"channel_after":after,"selection":selection_value(&selection),"success":true})).collect::<Vec<_>>(),
                    "finish_wall_us":finish_wall_us,"finish_cpu_us":finish_cpu_us})
            } else {
                let mut endpoint=Endpoint::open_with_policy(&config.store_dir,Limits::default(),ScopedAuthority(context.clone()),ColdGraphPolicy::Disabled)?;
                let mut committed=BTreeSet::<Id>::new(); let mut batches=Vec::new();
                while let Some(frames)=handle.block_on(channel.recv_batch(end,&cancel))? {
                    if batches.len()>=1024 {return Err(invalid("object session batch limit").into());}
                    let start=Instant::now(); let cpu=cpu_us()?; let mut replies=Vec::new();
                    for frame in &frames {
                        let reply=endpoint.handle(&context,frame,whole_start.elapsed().as_secs())?;
                        if reply.kind==103 {
                            committed.insert(reply.body.as_slice().try_into().map_err(|_|invalid("invalid published identity"))?);
                            if committed.len()>512 {return Err(invalid("object session publication limit").into());}
                        }
                        let terminal=matches!(reply.kind,103|127); replies.push(reply);
                        if terminal {break;}
                    }
                    let cpu_us=cpu_us()?-cpu; let wall_us=start.elapsed().as_micros();
                    let accepted=replies.last().is_some_and(|f|f.kind!=127);
                    batches.push((frames[0].transfer,frames.len(),replies.len(),wall_us,cpu_us,accepted));
                    handle.block_on(channel.send_batch(&replies,end,&cancel))?;
                    if !accepted {return Err(invalid("object receiver rejected transfer").into());}
                }
                drop(endpoint);
                let readback=Instant::now(); let read_cpu=cpu_us()?;
                let mut reopened=Endpoint::open_with_policy(&config.store_dir,Limits::default(),ScopedAuthority(context.clone()),ColdGraphPolicy::Disabled)?;
                let mut objects=Vec::new();
                for id in &committed {
                    let bytes=reopened.read_object(&context,*id)?;
                    if transfer::object_id(&bytes)!=*id {return Err(invalid("reopened object digest mismatch").into());}
                    objects.push((transfer::hex(id),bytes.len()));
                }
                for path in &config.verify_files {
                    let expected=read_bounded(path,transfer::client::MAX_BATCHED_OBJECT)?;
                    let id=transfer::object_id(&expected);
                    if !committed.contains(&id) || reopened.read_object(&context,id)?!=expected {
                        return Err(invalid("reopened exact fixture mismatch").into());
                    }
                }
                drop(reopened);
                let readback_wall_us=readback.elapsed().as_micros(); let readback_cpu_us=cpu_us()?-read_cpu;
                handle.block_on(channel.acknowledge_finish(end,&cancel))?;
                json!({"server_batches":batches.into_iter().map(|(id,request,reply,wall,cpu,accepted)|json!({"transfer_id":transfer::hex(&id),"request_frames":request,"response_frames":reply,"wall_us":wall,"cpu_us":cpu,"accepted":accepted})).collect::<Vec<_>>(),
                    "server_batch_timing":"Endpoint.handle codec decode, validation and durable publication; excludes stream wait and SCWBAT framing decode, which remain in whole process totals",
                    "readback":{"objects":objects.into_iter().map(|(id,len)|json!({"object_id":id,"bytes":len,"digest_verified":true})).collect::<Vec<_>>(),"expected_files_exact":config.verify_files.len(),"wall_us":readback_wall_us,"cpu_us":readback_cpu_us,"reopened":true}})
            };
            summary["status"]=json!("ok"); summary["role"]=json!(config.role);
            summary["scope"]=json!(config.scope); summary["namespace"]=json!(transfer::hex(&context.namespace()?));
            summary["setup_wall_us"]=json!(setup_wall_us); summary["setup_cpu_us"]=json!(setup_cpu_us);
            summary["channel"]=serde_json::to_value(channel.counters())?;
            summary["udp"]=serde_json::to_value(channel.udp_counters())?;
            summary["process_cpu_us"]=json!(cpu_us()?-whole_cpu);
            summary["wall_us"]=json!(whole_start.elapsed().as_micros());
            summary["timing"]=json!("operation includes input read/hash, codec selection and commit reply; setup/group preparation/seeds and finish/readback separately retained; process CPU includes nested work; final JSON write excluded; UDP counters snapshot before endpoint close, packet capture includes close");
            write_stats(&stats,&summary)?;
            Ok(())
        }).await?
    }.await;
    signal.abort();
    result
}
