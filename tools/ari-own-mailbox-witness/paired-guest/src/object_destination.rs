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
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

struct Active {
    call: Rc<ObjectDelivery>,
    until: Instant,
}
struct Gate {
    boot: MailboxReaderBootBinding,
    active: RefCell<Option<Active>>,
    traffic: RefCell<ReaderTraffic>,
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
}
impl Session {
    fn start(call: Rc<ObjectDelivery>, until: Instant) -> Result<Self> {
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
            traffic: Default::default(),
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
        })
    }
}

enum Phase {
    Received,
    Prepared {
        submission: ObjectSubmissionV1,
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
}

impl GuestDestination {
    pub(super) fn run(&mut self, task: &Value, until: Instant) -> Result<Value> {
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
        if self.last_ack == Some(op) {
            return Ok(
                json!({"original_call_acknowledged":true,"already_completed":true,"seq":op.seq}),
            );
        }
        if let Some(s) = &self.session {
            ensure!(
                s.gate.boot.durable_object_pair_sha256_v1 == Some(op.pair),
                "different resident object pair"
            )
        }
        let mut ops = ReaderTraffic::default();
        if self.pending.is_none() {
            loop {
                ensure!(
                    Instant::now() < until,
                    "no original Call arrived before deadline"
                );
                if let Some(delivery) = ObjectDelivery::receive_before(op.pair, until, &mut ops)? {
                    self.pending = Some(Pending {
                        call: Rc::new(delivery),
                        phase: Phase::Received,
                    });
                    break;
                }
                std::thread::sleep(
                    Duration::from_millis(5).min(until.saturating_duration_since(Instant::now())),
                )
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
            self.session = Some(Session::start(c.clone(), until)?);
        }
        let s = self.session.as_mut().context("original receiver missing")?;
        let context = c.object_context();
        let mut ack = ReaderTraffic::default();
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
                    submission: object_exchange::response_submission(c, &reply)?,
                    readback,
                };
            }
            if let Phase::Prepared {
                submission,
                readback,
            } = &mut p.phase
            {
                s.gate.check(&context)?;
                object_exchange::submit_before(op.pair, submission.clone(), until, &mut ops)?;
                p.phase = Phase::Accepted {
                    readback: readback.take(),
                };
            }
            c.acknowledge_before(until, &mut ack)?;
            Ok(())
        })();
        *s.gate.active.borrow_mut() = None;
        let auth = std::mem::take(&mut *s.gate.traffic.borrow_mut());
        action?;
        let Phase::Accepted { readback } = &p.phase else {
            anyhow::bail!("missing original Reply acceptance")
        };
        let result = json!({"original_call_acknowledged":true,"seq":op.seq,"transfer":hex::encode(op.transfer),"readback_hex":readback.as_ref().map(hex::encode),"operations_to_agent":ops.written,"operations_from_agent":ops.read,"ack_to_agent":ack.written,"ack_from_agent":ack.read,"authority_to_agent":auth.written,"authority_from_agent":auth.read});
        self.last_ack = Some(op);
        self.pending = None;
        Ok(result)
    }
}
