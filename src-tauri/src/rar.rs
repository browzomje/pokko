// RAR5 stored archives: https://www.rarlab.com/technote.htm
// Images are already compressed. Stored mode needs no proprietary RAR executable.
use std::{
    io::{Read, Write},
    path::Path,
};
fn vint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut b = (value & 127) as u8;
        value >>= 7;
        if value != 0 {
            b |= 128;
        }
        out.push(b);
        if value == 0 {
            break;
        }
    }
}
fn header(file: &mut std::fs::File, body: &[u8]) -> Result<(), String> {
    let mut bytes = Vec::new();
    vint(&mut bytes, body.len() as u64);
    bytes.extend_from_slice(body);
    file.write_all(&crc32fast::hash(&bytes).to_le_bytes())
        .and_then(|_| file.write_all(&bytes))
        .map_err(|e| e.to_string())
}
pub fn create(
    source: &Path,
    destination: &Path,
    checkpoint: impl Fn() -> Result<(), String>,
) -> Result<(), String> {
    let tmp = destination.with_extension("cbr.part");
    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(b"Rar!\x1a\x07\x01\x00")
        .map_err(|e| e.to_string())?;
    header(&mut file, &[1, 0, 0])?;
    let paths = crate::export::images(source)?;
    if paths.is_empty() {
        return Err("Nessuna pagina da esportare".into());
    }
    for path in paths {
        checkpoint()?;
        let data = std::fs::read(&path).map_err(|e| e.to_string())?;
        let name = path
            .strip_prefix(source)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let mut body = vec![2, 2];
        vint(&mut body, data.len() as u64); // packed size
        vint(&mut body, 4); // data CRC present
        vint(&mut body, data.len() as u64);
        vint(&mut body, 0o100644); // Unix regular file
        body.extend_from_slice(&crc32fast::hash(&data).to_le_bytes());
        vint(&mut body, 0); // RAR5, stored
        vint(&mut body, 1); // Unix
        vint(&mut body, name.len() as u64);
        body.extend_from_slice(name.as_bytes());
        header(&mut file, &body)?;
        file.write_all(&data).map_err(|e| e.to_string())?;
    }
    checkpoint()?;
    header(&mut file, &[5, 0, 0])?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(tmp, destination).map_err(|e| e.to_string())
}
fn read_vint(data: &[u8], pos: &mut usize) -> Result<u64, String> {
    let mut value = 0;
    for shift in (0..=63).step_by(7) {
        let b = *data.get(*pos).ok_or("Header CBR troncato")?;
        *pos += 1;
        if shift == 63 && b > 1 {
            return Err("Intero CBR non valido".into());
        }
        value |= ((b & 127) as u64) << shift;
        if b & 128 == 0 {
            return Ok(value);
        }
    }
    Err("Intero CBR non valido".into())
}
pub fn verify(path: &Path, expected: usize) -> Result<(), String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut signature = [0; 8];
    file.read_exact(&mut signature).map_err(|e| e.to_string())?;
    if &signature != b"Rar!\x1a\x07\x01\x00" {
        return Err("Firma CBR non valida".into());
    }
    let mut count = 0;
    loop {
        let mut crc = [0; 4];
        file.read_exact(&mut crc).map_err(|e| e.to_string())?;
        let mut size_bytes = Vec::new();
        for _ in 0..3 {
            let mut b = [0];
            file.read_exact(&mut b).map_err(|e| e.to_string())?;
            size_bytes.push(b[0]);
            if b[0] & 128 == 0 {
                break;
            }
        }
        let mut p = 0;
        let len = read_vint(&size_bytes, &mut p)? as usize;
        if len > 2 * 1024 * 1024 {
            return Err("Header CBR troppo grande".into());
        }
        let mut body = vec![0; len];
        file.read_exact(&mut body).map_err(|e| e.to_string())?;
        size_bytes.extend_from_slice(&body);
        if crc32fast::hash(&size_bytes) != u32::from_le_bytes(crc) {
            return Err("Header CBR danneggiato".into());
        }
        let mut p = 0;
        let kind = read_vint(&body, &mut p)?;
        let flags = read_vint(&body, &mut p)?;
        if kind == 5 {
            break;
        }
        if kind == 1 {
            continue;
        }
        if kind != 2 || flags != 2 {
            return Err("Blocco CBR non supportato".into());
        }
        let packed = read_vint(&body, &mut p)? as usize;
        let fflags = read_vint(&body, &mut p)?;
        let unpacked = read_vint(&body, &mut p)? as usize;
        let _attrs = read_vint(&body, &mut p)?;
        if fflags != 4 || packed != unpacked || packed > 64 * 1024 * 1024 {
            return Err("Pagina CBR non valida".into());
        }
        let crc: u32 = u32::from_le_bytes(
            body.get(p..p + 4)
                .ok_or("CRC CBR mancante")?
                .try_into()
                .unwrap(),
        );
        p += 4;
        if read_vint(&body, &mut p)? != 0 {
            return Err("CBR compresso non supportato".into());
        }
        let mut bytes = vec![0; packed];
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        if crc32fast::hash(&bytes) != crc {
            return Err("Pagina CBR danneggiata".into());
        }
        crate::integrity::image(&bytes)?;
        count += 1;
    }
    if count != expected {
        return Err("CBR incompleto: conteggio pagine diverso".into());
    }
    Ok(())
}
