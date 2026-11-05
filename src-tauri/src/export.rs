use lopdf::{dictionary, Document, Object, Stream};
use std::path::{Path, PathBuf};

pub fn images(source: &Path) -> Result<Vec<PathBuf>, String> {
    let mut dirs = std::fs::read_dir(source)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    let mut result = Vec::new();
    for dir in dirs {
        let mut files = std::fs::read_dir(dir)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|e| {
                    ["jpg", "png", "webp", "gif", "avif"].contains(&e.to_string_lossy().as_ref())
                })
            })
            .collect::<Vec<_>>();
        files.sort();
        result.extend(files);
    }
    Ok(result)
}
pub fn pdf(
    source: &Path,
    destination: &Path,
    checkpoint: impl Fn(usize, usize) -> Result<(), String>,
) -> Result<usize, String> {
    let image_paths = images(source)?;
    let total_images = image_paths.len();
    if total_images == 0 {
        return Err("Nessuna pagina da esportare".into());
    }
    let mut doc = Document::with_version("1.5");
    let root = doc.new_object_id();
    let mut kids = Vec::new();
    for (idx, path) in image_paths.into_iter().enumerate() {
        checkpoint(idx + 1, total_images)?;

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        let is_jpeg = ext == "jpg" || ext == "jpeg";

        let mut direct_jpeg = false;
        let mut dims = None;
        if is_jpeg {
            if let Ok((w, h)) = image::image_dimensions(&path) {
                const MAX_W: u32 = 3600;
                const MAX_H: u32 = 2560;
                if w <= MAX_W && h <= MAX_H {
                    direct_jpeg = true;
                    dims = Some((w, h));
                }
            }
        }

        let (w, h, bytes) = if direct_jpeg {
            let (w, h) = dims.unwrap();
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            (w, h, bytes)
        } else {
            let mut reader = image::ImageReader::open(&path)
                .map_err(|e| e.to_string())?
                .with_guessed_format()
                .map_err(|e| e.to_string())?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(20000);
            limits.max_image_height = Some(20000);
            limits.max_alloc = Some(512 * 1024 * 1024);
            reader.limits(limits);
            let img = reader
                .decode()
                .map_err(|e| format!("Conversione PDF di {}: {e}", path.display()))?
                .to_rgb8();
            let (orig_w, orig_h) = img.dimensions();

            const MAX_HEIGHT: u32 = 2400;
            const MAX_WIDTH: u32 = 3600;
            let (target_w, target_h) = if orig_h > MAX_HEIGHT || orig_w > MAX_WIDTH {
                let ratio_w = MAX_WIDTH as f32 / orig_w as f32;
                let ratio_h = MAX_HEIGHT as f32 / orig_h as f32;
                let ratio = ratio_w.min(ratio_h);
                (
                    ((orig_w as f32 * ratio).round() as u32).max(1),
                    ((orig_h as f32 * ratio).round() as u32).max(1),
                )
            } else {
                (orig_w, orig_h)
            };

            let img = if (target_w, target_h) != (orig_w, orig_h) {
                image::imageops::resize(
                    &img,
                    target_w,
                    target_h,
                    image::imageops::FilterType::Triangle,
                )
            } else {
                img
            };
            let (w, h) = img.dimensions();
            let mut bytes = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85)
                .encode_image(&img)
                .map_err(|e| e.to_string())?;
            (w, h, bytes)
        };

        let image = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Image",
                "Width" => w,
                "Height" => h,
                "ColorSpace" => "DeviceRGB",
                "BitsPerComponent" => 8,
                "Filter" => "DCTDecode"
            },
            bytes,
        ));
        let content = doc.add_object(Stream::new(
            dictionary! {},
            format!("q {w} 0 0 {h} 0 0 cm /Scan Do Q").into_bytes(),
        ));
        let page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => root,
            "MediaBox" => vec![0.into(), 0.into(), w.into(), h.into()],
            "Resources" => dictionary! {
                "XObject" => dictionary! { "Scan" => image }
            },
            "Contents" => content
        });
        kids.push(Object::Reference(page));
    }
    let pages_count = kids.len();
    doc.objects.insert(
        root,
        dictionary! {
            "Type" => "Pages",
            "Count" => pages_count as i64,
            "Kids" => kids
        }
        .into(),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => root });
    doc.trailer.set("Root", catalog);
    checkpoint(pages_count, pages_count)?;
    let tmp = destination.with_extension("pdf.part");
    doc.save(&tmp).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, destination).map_err(|e| e.to_string())?;
    Ok(pages_count)
}
