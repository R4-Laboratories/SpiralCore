//! Native Linux one-shot Zstandard control for the prepared 4096-byte object.
//! Each iteration starts a fresh independent frame and verifies the decoded bytes.
use std::{fs, time::Instant};
#[link(name="zstd")]
unsafe extern "C" {
    fn ZSTD_versionString()->*const std::ffi::c_char;
    fn ZSTD_compressBound(size:usize)->usize;
    fn ZSTD_compress(dst:*mut u8, dst_capacity:usize, src:*const u8,size:usize,level:i32)->usize;
    fn ZSTD_decompress(dst:*mut u8, capacity:usize, src:*const u8,size:usize)->usize;
    fn ZSTD_isError(code:usize)->u32;
}
fn main(){
    let (input,level)=(std::env::args().nth(1).unwrap(),std::env::args().nth(2).unwrap().parse::<i32>().unwrap());
    let src=fs::read(input).unwrap();
    assert_eq!(src.len(),4096);
    let mut compressed=vec![0;unsafe{ZSTD_compressBound(src.len())}];
    let mut restored=vec![0;src.len()];
    let mut len=0;
    let count=8192;
    let begin=Instant::now();
    for i in 0..count {
        let n=unsafe{ZSTD_compress(compressed.as_mut_ptr(),compressed.len(),src.as_ptr(),src.len(),level)};
        assert_eq!(unsafe{ZSTD_isError(n)},0);
        if i==0 { len=n; } else { assert_eq!(len,n); }
        let d=unsafe{ZSTD_decompress(restored.as_mut_ptr(),restored.len(),compressed.as_ptr(),n)};
        assert_eq!(d,src.len()); assert_eq!(restored,src);
    }
    let ns=begin.elapsed().as_nanos();
    let ver=unsafe{std::ffi::CStr::from_ptr(ZSTD_versionString())}.to_str().unwrap();
    println!("{ver} level={level} raw=4096 frame={len} decodes_verified={count} elapsed_ns={ns}");
}
