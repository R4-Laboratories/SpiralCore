use spiralcore_transfer::{zstd_dictionary as dict, Error};

fn samples() -> Vec<Vec<u8>> {
    (0..512).map(|i| format!("{{\"kind\":\"temperature-reading\",\"host\":\"node-{i:05}\",\"sequence\":{i},\"status\":\"healthy\",\"site\":\"building-a\",\"units\":\"celsius\",\"message\":\"all systems operating normally\"}}\n").into_bytes()).collect()
}

#[test]
fn trained_dictionary_exactly_recovers_held_out_and_arbitrary_bytes() {
    let training = samples();
    let refs: Vec<_> = training.iter().map(Vec::as_slice).collect();
    let artifact = dict::train(&refs, 8192).unwrap();
    assert_eq!(artifact, dict::train(&refs, 8192).unwrap());
    assert!(artifact.len() <= 8204);
    assert_eq!(&artifact[..8], b"SCZDICT1");
    assert!(zstd::zstd_safe::get_dict_id_from_dict(&artifact[12..]).is_some());
    for target in [
        vec![],
        vec![0],
        b"\0trailing\0\0".to_vec(),
        b"{ \"sequence\":9e1,\"new\":true,\"escaped\":\"\\u0061\"}\r\n".to_vec(),
        training[5].repeat(7000),
        (0..65537).map(|i| (i * 97) as u8).collect(),
    ] {
        let encoded = dict::encode(&artifact, &target).unwrap();
        assert_eq!(
            dict::decode(&artifact, &encoded, target.len()).unwrap(),
            target
        );
        assert!(dict::decode(&artifact, &encoded, target.len() + 1).is_err());
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(dict::decode(&artifact, &trailing, target.len()).is_err());
        let mut damaged = encoded;
        *damaged.last_mut().unwrap() ^= 1;
        assert!(dict::decode(&artifact, &damaged, target.len()).is_err());
    }
}

#[test]
fn dictionary_rejects_malformed_artifacts_and_resource_expansion() {
    let training = samples();
    let refs: Vec<_> = training.iter().map(Vec::as_slice).collect();
    let artifact = dict::train(&refs, 8192).unwrap();
    assert_eq!(dict::train(&refs, 65536), Err(Error::Limit));
    assert_eq!(
        dict::train(&vec![b"x".as_slice(); 4097], 8192),
        Err(Error::Limit)
    );
    assert_eq!(
        dict::train(&[&vec![0; (4 << 20) + 1]], 8192),
        Err(Error::Limit)
    );
    for mut bad in [
        vec![],
        artifact[..11].to_vec(),
        artifact[..artifact.len() - 1].to_vec(),
        artifact.clone(),
    ] {
        if bad.len() == artifact.len() {
            bad[0] ^= 1;
        }
        assert!(dict::encode(&bad, b"unsupported").is_err());
    }
    let mut trailing = artifact.clone();
    trailing.push(0);
    assert!(dict::encode(&trailing, b"").is_err());
    assert_eq!(
        dict::decode(&artifact, &[], (16 << 20) + 1),
        Err(Error::Limit)
    );
    let oversized_window = [0x28, 0xb5, 0x2f, 0xfd, 0x44, 160, 0, 0];
    assert_eq!(
        dict::decode(&artifact, &oversized_window, 1),
        Err(Error::Limit)
    );
}
