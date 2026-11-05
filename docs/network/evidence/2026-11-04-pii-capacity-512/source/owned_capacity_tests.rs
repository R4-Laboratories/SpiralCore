//! Finite private-builder capacity sweep; no production controls API.
use super::*;
use crate::policy::PiiPolicy;

fn orders() -> Vec<Vec<usize>> {
    fn visit(prefix: &mut Vec<usize>, used: u8, all: &mut Vec<Vec<usize>>) {
        for index in 0..5 {
            if used & (1 << index) == 0 {
                prefix.push(index);
                all.push(prefix.clone());
                visit(prefix, used | (1 << index), all);
                prefix.pop();
            }
        }
    }
    let mut all = Vec::with_capacity(325);
    visit(&mut Vec::with_capacity(5), 0, &mut all);
    assert_eq!(all.len(), 325);
    all
}
fn bounded(pattern: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(32_768);
    while body.len() < 32_768 {
        body.extend_from_slice(&pattern[..pattern.len().min(32_768 - body.len())]);
    }
    while std::str::from_utf8(&body).is_err() {
        body.pop();
    }
    body.resize(32_768, b' ');
    body
}
fn candidate(policy: &PiiPolicy, nfa: usize) -> Result<PiiRedactor, PiiPolicyError> {
    let mut controls = PiiRedactor::owned_pii_requirements(policy)?;
    controls.nfa_size_limit_bytes = nfa;
    PiiRedactor::build_owned(PiiRedactor::owned_selection(policy)?, controls)
}

#[test]
fn fixed_owned_nfa_candidates_preserve_all_ordered_policies_and_search_samples() {
    let dense = bounded(b"synthetic@example.invalid 001-01-0001 4111111111111111 +12025550123 GB82WEST12345698765432\n");
    let cases = [
        dense,
        bounded("\u{03bb}\u{754c} no-at-domain 000-00-0000 1111111111111 +abcdef GB00INVALID000000000000 \n".as_bytes()),
        bounded(b"ordinary-synthetic-telemetry-without-sensitive-shaped-fields; "),
    ];
    let orders = orders();
    let mut accepted = 0;
    for nfa in [524_288, 262_144, 131_072] {
        let mut constructors = 0;
        let mut rejected = false;
        for (index, order) in orders.iter().enumerate() {
            let policy = PiiPolicy {
                mode: Some("redact".into()),
                categories: order
                    .iter()
                    .map(|&i| DEFAULT_RULES[i].name.to_owned())
                    .collect(),
            };
            let scanner = match candidate(&policy, nfa) {
                Ok(scanner) => scanner,
                Err(_) => {
                    println!(
                        "{{\"scope\":\"private-capacity-candidate\",\"nfa\":{nfa},\"accepted\":false,\"constructors_before_failure\":{constructors},\"failed_selection\":{},\"rule_indices\":{order:?}}}",
                        index + 1
                    );
                    rejected = true;
                    break;
                }
            };
            let legacy = PiiRedactor::from_policy(&policy).unwrap().unwrap();
            assert_eq!(
                scanner.redact(&cases[0]),
                legacy.redact(&cases[0]),
                "ordered policy output/names, cap={nfa}, selection={}",
                index + 1
            );
            constructors += 1;
        }
        if rejected {
            continue;
        }
        assert_eq!(constructors, 325);
        let policies = [
            PiiPolicy {
                mode: Some("redact".into()),
                categories: vec![],
            },
            PiiPolicy {
                mode: Some("redact".into()),
                categories: vec!["iban".into(), "email".into(), "credit_card".into()],
            },
        ];
        let mut searches = 0;
        for policy in &policies {
            let scanner = candidate(policy, nfa).expect("passing candidate sample policy");
            let legacy = PiiRedactor::from_policy(policy).unwrap().unwrap();
            for body in &cases {
                let expected = legacy.redact(body);
                assert_eq!(scanner.redact(body), expected);
                searches += 1;
                for _ in 0..8 {
                    assert_eq!(scanner.redact(body), expected);
                    searches += 1;
                }
            }
        }
        // Public policy/default semantics remain the same private builder path.
        for mode in [
            None,
            Some("detect"),
            Some("redact"),
            Some("refuse"),
            Some("disabled"),
        ] {
            let policy = PiiPolicy {
                mode: mode.map(str::to_owned),
                categories: vec![],
            };
            let scanner = candidate(&policy, nfa).expect("passing candidate default/mode");
            let legacy = PiiRedactor::from_policy(&policy)
                .unwrap()
                .unwrap_or_else(PiiRedactor::with_default_rules);
            assert_eq!(scanner.mode(), legacy.mode());
            assert_eq!(scanner.scan(&cases[0]), legacy.scan(&cases[0]));
            assert_eq!(
                scanner.match_spans(&cases[0]),
                legacy.match_spans(&cases[0])
            );
            assert_eq!(scanner.redact(&cases[0]), legacy.redact(&cases[0]));
        }
        accepted += 1;
        println!(
            "{{\"scope\":\"private-capacity-candidate\",\"nfa\":{nfa},\"accepted\":true,\"constructors\":{constructors},\"legacy_tuples_equal\":true,\"representative_searches\":{searches},\"default_mode_cases\":5}}"
        );
    }
    assert!(
        accepted > 0,
        "all preregistered candidates rejected; preserve original source"
    );
}
