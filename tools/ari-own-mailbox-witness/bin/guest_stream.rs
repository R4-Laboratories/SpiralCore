pub(super) const SCOPE: &str = "mvm/owned-mailbox/guest-5257/post-mux-host-stream/v1";
pub(super) const NAME: &str = "mailbox-guest-stream-v1.counts";

// A prefix can still identify the last protocol phase. Publish totals only
// for paired begin/end records, without truncation, overflow or sequence gaps.
pub(super) fn completed(raw: &str) -> Option<(u64, u64, u64, bool)> {
    completed_for(raw, SCOPE).map(|(n, r, w, _, _, finished)| (n, r, w, finished))
}

pub(super) fn completed_for(raw: &str, scope: &str) -> Option<(u64, u64, u64, u64, u64, bool)> {
    let mut lines = raw.strip_suffix('\n')?.lines();
    if lines.next()? != scope {
        return None;
    }
    let (mut exchanges, mut read, mut written) = (0_u64, 0_u64, 0_u64);
    let (mut read_errors, mut write_errors) = (0u64, 0u64);
    let mut active = None;
    let mut finished = false;
    for line in lines {
        if finished {
            return None;
        }
        let mut fields = line.split(' ');
        let kind = fields.next()?;
        let seq: u64 = fields.next()?.parse().ok()?;
        match kind {
            "begin" if active.is_none() && seq == exchanges + 1 && seq <= 64 => active = Some(seq),
            "finish" if active.is_none() && seq == exchanges => finished = true,
            "phase" if active == Some(seq) => match fields.next()? {
                "submit_verdict"
                | "reader_frame"
                | "challenge_written"
                | "possession_received"
                | "lease_created"
                | "ack_committed"
                | "reader_refused"
                | "reader_empty"
                | "reader_challenge" => (),
                _ => return None,
            },
            "end" if active == Some(seq) => {
                read = read.checked_add(fields.next()?.parse().ok()?)?;
                written = written.checked_add(fields.next()?.parse().ok()?)?;
                read_errors = read_errors.checked_add(fields.next()?.parse().ok()?)?;
                write_errors = write_errors.checked_add(fields.next()?.parse().ok()?)?;
                if fields.next()? != "false" {
                    return None;
                }
                exchanges = seq;
                active = None;
            }
            _ => return None,
        }
        if fields.next().is_some() {
            return None;
        }
    }
    ((finished || exchanges > 0) && active.is_none()).then_some((
        exchanges,
        read,
        written,
        read_errors,
        write_errors,
        finished,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_verified_exchanges_in_each_direction() {
        let raw = format!(
            "{SCOPE}\nbegin 1\nphase 1 submit_verdict\nend 1 8 15 0 1 false\nbegin 2\nphase 2 reader_frame\nend 2 11 32 1 0 false\n"
        );
        assert_eq!(completed(&raw), Some((2, 19, 47, false)));
    }

    #[test]
    fn only_a_matching_final_record_closes_the_observation() {
        let raw = format!("{SCOPE}\nbegin 1\nend 1 9 8 0 0 false\n");
        assert_eq!(
            completed(&format!("{raw}finish 1\n")),
            Some((1, 9, 8, true))
        );
        assert_eq!(
            completed(&format!("{SCOPE}\nfinish 0\n")),
            Some((0, 0, 0, true))
        );
        assert_eq!(completed(&format!("{raw}finish 2\n")), None);
        assert_eq!(completed(&format!("{raw}finish 1\nbegin 2\n")), None);
    }

    #[test]
    fn incomplete_or_ambiguous_observations_have_no_total() {
        for rows in [
            "",
            "begin 1\n",
            "begin 2\nend 2 0 0 0 0 false\n",
            "begin 1\nend 1 0 0 0 0 true\n",
            "begin 1\nend 1 1 2 0 0 false",
            "begin 1\nphase 2 reader_frame\nend 1 0 0 0 0 false\n",
            "begin 1\nend 1 1 3 0 0 false\noverflow\n",
            "begin 1\nend 1 18446744073709551615 0 0 0 false\nbegin 2\nend 2 1 0 0 0 false\n",
        ] {
            assert_eq!(completed(&format!("{SCOPE}\n{rows}")), None);
        }
    }
}
