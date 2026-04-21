use pdf_oxide::api::Pdf;
use std::fs::File;
use std::io::{self, Read, Write};

fn print_help(name: &str) {
    eprintln!("Usage: {} [OPTIONS] <FILENAME>", name);
    eprintln!();
    eprintln!(
        "Converts documents to markdown. Reads the input file and outputs markdown to stdout."
    );
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  <FILENAME>         Path to the input file to convert.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --help, -h       Show this help message and exit.");
    eprintln!("  --tail <N>       Limit output to the last N lines of the result.");
    eprintln!("  --output, -o <FILE>  Write output to FILE instead of stdout.");
    eprintln!();
    eprintln!("Supported formats:");
    eprintln!("  PDF              Portable Document Format (.pdf)");
    eprintln!("  DOCX             Microsoft Word (.docx)");
    eprintln!("  XLSX             Microsoft Excel (.xlsx)");
    eprintln!("  PPTX             Microsoft PowerPoint (.pptx)");
    eprintln!("  RSS / Atom       Feed files (.rss, .atom, .xml)");
    eprintln!("  Images           EXIF metadata extraction (.jpg, .jpeg)");
    eprintln!(
        "  Audio            Metadata placeholder (.mp3, .wav, .ogg, .m4a, .aac, .flac, .weba)"
    );
    eprintln!("  Video            Metadata placeholder (.mp4, .mov, .avi, .wmv, .flv, .webm, .mkv, .m4v, .mpg, .mpeg)");
    eprintln!("  7z               Archive extraction (.7z)");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} document.pdf", name);
    eprintln!("  {} --output out.md document.docx", name);
    eprintln!("  {} --tail 10 largefile.pdf", name);
}

fn extract_pdf_text(bytes: &[u8]) -> Result<String, String> {
    let mut doc =
        Pdf::from_bytes(bytes.to_vec()).map_err(|e| format!("Failed to load PDF: {}", e))?;
    let page_count = doc
        .page_count()
        .map_err(|e| format!("Failed to get page count: {}", e))?;
    let mut result = String::new();
    for i in 0..page_count {
        let page_text = doc
            .to_text(i)
            .map_err(|e| format!("Failed to extract PDF text: {}", e))?;
        if !page_text.trim().is_empty() {
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str(&page_text);
        }
    }
    if result.is_empty() {
        Err("No text content found in PDF".to_string())
    } else {
        Ok(result)
    }
}

fn convert_feed(bytes: &[u8]) -> Result<String, String> {
    let feed = feed_rs::parser::parse(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Failed to parse feed: {}", e))?;

    let mut markdown = String::new();
    if let Some(title) = &feed.title {
        markdown.push_str(&format!("# {}\n\n", title.content));
    }

    for entry in &feed.entries {
        if let Some(title) = &entry.title {
            markdown.push_str(&format!("## {}\n\n", title.content));
        }
        if let Some(summary) = &entry.summary {
            markdown.push_str(&html2md::parse_html(&summary.content));
            markdown.push_str("\n\n");
        }
        if let Some(content) = &entry.content {
            if let Some(body) = &content.body {
                markdown.push_str(&html2md::parse_html(body));
                markdown.push_str("\n\n");
            }
        }
        if let Some(updated) = &entry.updated {
            markdown.push_str(&format!("_Updated: {}_\n\n", updated));
        }
    }
    Ok(markdown)
}

fn extract_exif(bytes: &[u8]) -> Result<String, String> {
    let mut file = std::io::Cursor::new(bytes);
    let reader = exif::Reader::new();
    let exif = reader
        .read_from_container(&mut file)
        .map_err(|e| format!("Failed to read EXIF data: {}", e))?;

    let mut markdown = String::new();
    markdown.push_str("# Image Metadata\n\n");
    for field in exif.fields() {
        markdown.push_str(&format!(
            "**{}**: {}\n",
            field.tag,
            field.display_value().with_unit(&exif)
        ));
    }
    Ok(markdown)
}

fn extract_7z(bytes: &[u8], filename: &str) -> Result<String, String> {
    let temp_dir = std::env::temp_dir().join(format!("opencode-7z-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let cursor = std::io::Cursor::new(bytes);
    sevenz_rust::decompress(cursor, &temp_dir)
        .map_err(|e| format!("Failed to extract 7z archive: {}", e))?;

    let mut markdown = String::new();
    markdown.push_str(&format!("# Archive: {}\n\n", filename));

    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Ok(data) = std::fs::read(&path) {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    match convert_to_markdown(&data, name) {
                        Ok(content) => {
                            markdown.push_str(&format!("## {}\n\n{}\n\n", name, content));
                        }
                        Err(e) => {
                            markdown.push_str(&format!("## {}\n\nError: {}\n\n", name, e));
                        }
                    }
                }
            }
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(markdown)
}

fn convert_to_markdown(bytes: &[u8], filename: &str) -> Result<String, String> {
    let lower = filename.to_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");

    match ext {
        "pdf" => extract_pdf_text(bytes),
        "rss" | "atom" => convert_feed(bytes),
        "jpg" | "jpeg" => extract_exif(bytes),
        "7z" => extract_7z(bytes, filename),
        "png" | "gif" | "bmp" | "tiff" | "tif" | "webp" | "svg" => {
            Ok(format!("# Image\n\n![{}]({})\n", filename, filename))
        }
        "mp3" | "wav" | "ogg" | "m4a" | "aac" | "flac" | "weba" => Ok(format!(
            "# Audio\n\nAudio file: `{}`\n\n<!-- Audio transcription not available -->\n",
            filename
        )),
        "mp4" | "mov" | "avi" | "wmv" | "flv" | "webm" | "mkv" | "m4v" | "mpg" | "mpeg" => {
            Ok(format!(
                "# Video\n\nVideo file: `{}`\n\n<!-- Video transcription not available -->\n",
                filename
            ))
        }
        _ => {
            let mut input = markdownify::MarkdownifyInput::from_bytes(bytes, filename.to_string())
                .map_err(|e| format!("Failed to parse file: {}", e))?;

            if let Some(ext) = input.id.rsplit('.').next() {
                input.set_ext(ext.to_lowercase());
            }

            input
                .convert()
                .map_err(|e| format!("Failed to convert to markdown: {}", e))
        }
    }
}

fn tail_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= n {
        text.to_string()
    } else {
        lines[lines.len() - n..].join("\n")
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut tail_lines_count: Option<usize> = None;
    let mut output_file: Option<&str> = None;
    let mut filename: Option<&str> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help(&args[0]);
                return;
            }
            "--tail" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --tail requires a number argument");
                    std::process::exit(1);
                }
                tail_lines_count = Some(args[i].parse().unwrap_or_else(|_| {
                    eprintln!("Error: Invalid number for --tail: {}", args[i]);
                    std::process::exit(1);
                }));
            }
            "--output" | "-o" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --output requires a file path argument");
                    std::process::exit(1);
                }
                output_file = Some(&args[i]);
            }
            _ => {
                if filename.is_none() {
                    filename = Some(&args[i]);
                } else {
                    eprintln!("Error: Unexpected argument: {}", args[i]);
                    std::process::exit(1);
                }
            }
        }
        i += 1;
    }

    let filename = match filename {
        Some(f) => f,
        None => {
            print_help(&args[0]);
            std::process::exit(1);
        }
    };

    let bytes = if atty::is(atty::Stream::Stdin) {
        let mut file = File::open(filename).unwrap_or_else(|e| {
            eprintln!("Failed to open file '{}': {}", filename, e);
            std::process::exit(1);
        });
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap_or_else(|e| {
            eprintln!("Failed to read file '{}': {}", filename, e);
            std::process::exit(1);
        });
        buf
    } else {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).unwrap_or_else(|e| {
            eprintln!("Failed to read stdin: {}", e);
            std::process::exit(1);
        });
        buf
    };

    match convert_to_markdown(&bytes, filename) {
        Ok(markdown) => {
            let output = match tail_lines_count {
                Some(n) => tail_lines(&markdown, n),
                None => markdown,
            };
            match output_file {
                Some(path) => {
                    let mut file = std::fs::File::create(path).unwrap_or_else(|e| {
                        eprintln!("Failed to create file '{}': {}", path, e);
                        std::process::exit(1);
                    });
                    if let Err(e) = file.write_all(output.as_bytes()) {
                        eprintln!("Failed to write to file '{}': {}", path, e);
                        std::process::exit(1);
                    }
                }
                None => {
                    if let Err(e) = io::stdout().write_all(output.as_bytes()) {
                        eprintln!("Failed to write output: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
