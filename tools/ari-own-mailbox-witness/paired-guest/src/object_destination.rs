//! One retained original destination, one outstanding Call, and the common
//! SmallV1 object receiver. Retry the immutable Reply until its original V2,
//! then retry only the exact signed Call ACK.
use super::{object_exchange, object_reader::ObjectDelivery};
use anyhow::{Context as _, Result, ensure};
use mvm_core::mailbox::{
    reader::{MailboxReaderBootBinding, verify_object_call_authority},
    reader_wire::{
        ApplicationReaderRequest, ObjectSubmissionV1, read_response, write_application_request,
    },
};
use serde_json::{Value, json};
use spiralcore_mvm_guest::{ReaderTraffic, application_reader_before};
use spiralcore_transfer::{
    Error, Id,
    endpoint::{Authority, Context, Endpoint},
    resource_profile::ResourceProfile,
};
use std::{
    cell::RefCell,
    fs::{DirBuilder, File},
    io::Read,
    os::unix::fs::DirBuilderExt,
    rc::Rc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

struct Active {
    call: Rc<ObjectDelivery>,
    until: Instant,
}
struct Gate {
    boot: MailboxReaderBootBinding,
    active: RefCell<Option<Active>>,
    traffic: Rc<RefCell<ReaderTraffic>>,
}
impl Gate {
    fn check(&self, c: &Context) -> Result<()> {
        let active = self.active.borrow();
        let Active { call, until } = active.as_ref().context("no selected original Call")?;
        ensure!(
            Instant::now() < *until
                && &call.object_context() == c
                && MailboxReaderBootBinding::load_guest_original()? == self.boot,
            "original object receiver scope changed"
        );
        let mut challenge = [0; 32];
        File::open("/dev/urandom")?.read_exact(&mut challenge)?;
        let mut meter = self.traffic.borrow_mut();
        let mut io = application_reader_before(*until, &mut meter)?;
        write_application_request(
            &mut io,
            &ApplicationReaderRequest::AuthorizeObjectCallV1 {
                pair: call.pair(),
                lease: Box::new(call.lease().clone()),
                challenge,
            },
        )?;
        let verdict = read_response(&mut io)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_millis()
            .try_into()?;
        verify_object_call_authority(
            &self.boot,
            &call.pair(),
            call.lease(),
            call.header(),
            &call.frame().encode()?,
            &challenge,
            &verdict,
            now,
        )?;
        ensure!(
            Instant::now() < *until,
            "object authority reply arrived after deadline"
        );
        Ok(())
    }
}
struct OriginalAuthority(Rc<Gate>);
impl Authority for OriginalAuthority {
    fn authorize(&self, c: &Context) -> spiralcore_transfer::Result<()> {
        self.0.check(c).map_err(|_| Error::Unauthorized)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Operation {
    pair: Id,
    transfer: Id,
    seq: u64,
}

struct Session {
    gate: Rc<Gate>,
    endpoint: Endpoint<OriginalAuthority>,
    since: Instant,
    instance: [u8; 16],
}
impl Session {
    fn start(
        call: Rc<ObjectDelivery>,
        until: Instant,
        traffic: Rc<RefCell<ReaderTraffic>>,
    ) -> Result<Self> {
        let mut instance = [0u8; 16];
        File::open("/dev/urandom")?.read_exact(&mut instance)?;
        let boot = MailboxReaderBootBinding::load_guest_original()?;
        ensure!(
            Some(call.pair()) == boot.durable_object_pair_sha256_v1,
            "original boot selected a different pair"
        );
        let gate = Rc::new(Gate {
            boot,
            active: RefCell::new(Some(Active {
                call: call.clone(),
                until,
            })),
            traffic,
        });
        gate.check(&call.object_context())?;
        // Exclusive for this boot. Process loss leaves the original private
        // directory in place and refuses silent reattachment to cached bytes.
        let name = format!(
            "/tmp/spiralcore-object-{}-{}",
            hex::encode(call.pair()),
            hex::encode(gate.boot.boot.as_bytes())
        );
        DirBuilder::new().mode(0o700).create(&name)?;
        let profile = ResourceProfile::SmallV1;
        let endpoint = Endpoint::open_with_resource_profile(
            name.as_ref(),
            profile.receiver_limits(),
            OriginalAuthority(gate.clone()),
            profile,
        )?;
        Ok(Self {
            gate,
            endpoint,
            since: Instant::now(),
            instance,
        })
    }
}

enum Phase {
    Received,
    Computed {
        reply: spiralcore_transfer::protocol::Frame,
    },
    Prepared {
        submission: ObjectSubmissionV1,
        refusal: Option<mvm_core::mailbox::frame::Refusal>,
        readback: Option<Vec<u8>>,
    },
    Accepted {
        readback: Option<Vec<u8>>,
    },
}
struct Pending {
    call: Rc<ObjectDelivery>,
    phase: Phase,
}
#[derive(Default)]
pub(super) struct GuestDestination {
    session: Option<Session>,
    pending: Option<Pending>,
    last_ack: Option<Operation>,
    operations: ReaderTraffic,
    acknowledgments: ReaderTraffic,
    authority: Rc<RefCell<ReaderTraffic>>,
}

impl GuestDestination {
    pub(super) fn run(&mut self, task: &Value, until: Instant) -> Result<Value> {
        let before = self.counts();
        let result = self.run_turn(task, until);
        let total = self.counts();
        let counters = json!({"this_invocation":std::array::from_fn::<u64,6,_>(|n|total[n]-before[n]),"worker_cumulative":total,"order":["operations_to_agent","operations_from_agent","ack_to_agent","ack_from_agent","authority_to_agent","authority_from_agent"]});
        match result {
            Ok(mut value) => {
                let s = self
                    .session
                    .as_ref()
                    .context("original destination session missing")?;
                value["receiver_instance"] = json!(hex::encode(s.instance));
                value["receiver_boot"] = json!(hex::encode(s.gate.boot.boot.as_bytes()));
                value["reader_bytes"] = counters;
                Ok(value)
            }
            Err(e) => Err(e.context(format!("destination reader bytes {counters}"))),
        }
    }
    fn counts(&self) -> [u64; 6] {
        let a = self.authority.borrow();
        [
            self.operations.written,
            self.operations.read,
            self.acknowledgments.written,
            self.acknowledgments.read,
            a.written,
            a.read,
        ]
    }
    fn run_turn(&mut self, task: &Value, until: Instant) -> Result<Value> {
        let op = Operation {
            pair: crate::field(task, "pair")?,
            transfer: crate::field(task, "transfer")?,
            seq: task["seq"]
                .as_u64()
                .context("expected original frame sequence")?,
        };
        ensure!(
            task.as_object().is_some_and(|m| m.len() == 3),
            "unexpected receiver input"
        );
        if let Some(s) = &self.session {
            ensure!(
                s.gate.boot.durable_object_pair_sha256_v1 == Some(op.pair)
                    && MailboxReaderBootBinding::load_guest_original()? == s.gate.boot,
                "different resident object pair or boot"
            );
        }
        if self.last_ack == Some(op) {
            return Ok(
                json!({"original_call_acknowledged":true,"already_completed":true,"seq":op.seq,"readback_status":"new_authorized_call_required"}),
            );
        }
        if self.pending.is_none() {
            let mut empty = crate::object_reader::EmptyPollDelay::default();
            loop {
                ensure!(
                    Instant::now() < until,
                    "no original Call arrived before deadline"
                );
                if let Some(delivery) =
                    ObjectDelivery::receive_before(op.pair, until, &mut self.operations)?
                {
                    self.pending = Some(Pending {
                        call: Rc::new(delivery),
                        phase: Phase::Received,
                    });
                    break;
                }
                empty.wait(until);
            }
        }
        let p = self.pending.as_mut().context("original delivery missing")?;
        let c = &p.call;
        let f = c.frame();
        ensure!(
            c.pair() == op.pair
                && (f.transfer, f.seq) == (op.transfer, op.seq)
                && c.header().kind == mvm_contract::protocol::mailbox::MailboxMessageKind::Call
                && c.header().message_id == object_exchange::call_id(&op.pair, f)
                && c.header().correlation_id == object_exchange::correlation(&op.pair, f),
            "different pending original Call"
        );
        if self.session.is_none() {
            self.session = Some(Session::start(c.clone(), until, self.authority.clone())?);
        }
        let s = self.session.as_mut().context("original receiver missing")?;
        let context = c.object_context();
        *s.gate.active.borrow_mut() = Some(Active {
            call: c.clone(),
            until,
        });
        let action = (|| -> Result<()> {
            if matches!(p.phase, Phase::Received) {
                s.gate.check(&context)?;
                let reply = match s.endpoint.handle(&context, f, s.since.elapsed().as_secs()) {
                    Ok(response) => response,
                    Err(Error::Unauthorized) => {
                        anyhow::bail!("original object authority unavailable")
                    }
                    Err(e) => f.reply(127, (e as u16).to_le_bytes().to_vec()),
                };
                p.phase = Phase::Computed { reply };
            }
            if let Phase::Computed { reply } = &p.phase {
                let readback = if reply.kind == 103 {
                    let id: Id = reply.body.as_slice().try_into()?;
                    let bytes = s.endpoint.read_object(&context, id)?;
                    ensure!(
                        spiralcore_transfer::object_id(&bytes) == id,
                        "published object readback differs"
                    );
                    Some(bytes)
                } else {
                    None
                };
                p.phase = Phase::Prepared {
                    submission: object_exchange::response_submission(c, reply)?,
                    refusal: None,
                    readback,
                };
            }
            if let Phase::Prepared {
                submission,
                refusal,
                readback,
            } = &mut p.phase
            {
                if let Some(reason) = refusal {
                    return Err(anyhow::anyhow!("original Reply was refused: {reason}"));
                }
                s.gate.check(&context)?;
                if let Err(e) = object_exchange::submit_before(
                    op.pair,
                    submission.clone(),
                    until,
                    &mut self.operations,
                ) {
                    if let Some(decoded) = e
                        .downcast_ref::<object_exchange::SubmissionRefused>()
                        .filter(|refused| !refused.retryable())
                    {
                        *refusal = Some(decoded.0.clone());
                    }
                    return Err(e);
                }
                p.phase = Phase::Accepted {
                    readback: readback.take(),
                };
            }
            c.acknowledge_before(until, &mut self.acknowledgments)?;
            Ok(())
        })();
        *s.gate.active.borrow_mut() = None;
        action?;
        let Phase::Accepted { readback } = &p.phase else {
            anyhow::bail!("missing original Reply acceptance")
        };
        // Report only scalar framing metadata of the original handled Call.
        let mut result = json!({"original_call_acknowledged":true,"seq":op.seq,"transfer":hex::encode(op.transfer),"readback_hex":readback.as_ref().map(hex::encode)});
        result["frame_kind"] = json!(f.kind);
        if f.kind == 1 || f.kind == 2 {
            result["wire_mode"] = json!(f.body.first());
        }
        if f.kind == 2 {
            let len = f
                .body
                .get(1..5)
                .and_then(|b| <[u8; 4]>::try_from(b).ok())
                .map(u32::from_le_bytes);
            result["data_payload_bytes"] = json!(len);
        }
        self.last_ack = Some(op);
        self.pending = None;
        Ok(result)
    }
}
