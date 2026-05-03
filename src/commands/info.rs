// =============================================================================
// fqc-rust - Info Command
// =============================================================================

use crate::error::Result;
use crate::fqc_reader::{ArchiveInfo, FqcReader};
use crate::types::decode_codec_family;

// =============================================================================
// InfoOptions
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct InfoOptions {
    pub input_path: String,
    pub json: bool,
    pub detailed: bool,
    /// Show codec information for each block
    pub show_codecs: bool,
}

// =============================================================================
// InfoCommand
// =============================================================================

pub struct InfoCommand {
    opts: InfoOptions,
}

impl InfoCommand {
    pub fn new(opts: InfoOptions) -> Self {
        Self { opts }
    }

    pub fn execute(self) -> i32 {
        match self.run() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Info failed: {e}");
                e.exit_code_num()
            }
        }
    }

    fn run(&self) -> Result<()> {
        let mut reader = FqcReader::open(&self.opts.input_path)?;
        let info = reader.info();

        if self.opts.json {
            self.print_json(&info);
        } else {
            self.print_human(&info, &mut reader)?;
        }

        Ok(())
    }

    fn print_json(&self, info: &ArchiveInfo) {
        println!("{{");
        println!("  \"file\": \"{}\",", info.file_path);
        println!("  \"file_size\": {},", info.file_size);
        println!("  \"total_reads\": {},", info.total_reads);
        println!("  \"num_blocks\": {},", info.num_blocks);
        println!("  \"original_filename\": \"{}\",", info.original_filename);
        println!("  \"timestamp\": {},", info.timestamp);
        println!("  \"is_paired\": {},", info.is_paired);
        println!("  \"has_reorder_map\": {},", info.has_reorder_map);
        println!("  \"preserve_order\": {},", info.preserve_order);
        println!("  \"streaming_mode\": {},", info.streaming_mode);
        println!("  \"quality_mode\": \"{}\",", info.quality_mode.as_str());
        println!("  \"id_mode\": \"{}\",", info.id_mode.as_str());
        println!("  \"pe_layout\": \"{}\",", info.pe_layout.as_str());
        println!("  \"read_length_class\": \"{}\"", info.read_length_class.as_str());
        println!("}}");
    }

    fn print_human(&self, info: &ArchiveInfo, reader: &mut FqcReader) -> Result<()> {
        println!("File:              {}", info.file_path);
        println!("File size:         {} bytes", info.file_size);
        println!("Total reads:       {}", info.total_reads);
        println!("Num blocks:        {}", info.num_blocks);
        println!("Original filename: {}", info.original_filename);
        println!("Is paired-end:     {}", info.is_paired);
        println!("Has reorder map:   {}", info.has_reorder_map);
        println!("Preserve order:    {}", info.preserve_order);
        println!("Streaming mode:    {}", info.streaming_mode);
        println!("Quality mode:      {}", info.quality_mode.as_str());
        println!("ID mode:           {}", info.id_mode.as_str());
        println!("PE layout:         {}", info.pe_layout.as_str());
        println!("Read length class: {}", info.read_length_class.as_str());

        if self.opts.detailed {
            println!("\nBlock Index:");
            println!(
                "  {:>6}  {:>12}  {:>12}  {:>10}  {:>10}",
                "Block", "Offset", "CompSize", "ArchiveID", "Reads"
            );
            for (i, entry) in reader.block_index.entries.iter().enumerate() {
                println!(
                    "  {:>6}  {:>12}  {:>12}  {:>10}  {:>10}",
                    i, entry.offset, entry.compressed_size, entry.archive_id_start, entry.read_count
                );
            }
        }

        if self.opts.show_codecs {
            let num_blocks = reader.block_count();
            println!("\nBlock Codecs:");
            println!(
                "  {:>6}  {:>12}  {:>12}  {:>12}  {:>12}",
                "Block", "IDs", "Seq", "Qual", "Aux"
            );
            for i in 0..num_blocks {
                if let Ok(bh) = reader.read_block_header(i as u32) {
                    let fmt_codec = |c: u8| -> String {
                        let family = decode_codec_family(c);
                        let version = c & 0x0F;
                        format!("{:?}v{}", family, version)
                    };
                    println!(
                        "  {:>6}  {:>12}  {:>12}  {:>12}  {:>12}",
                        i,
                        fmt_codec(bh.codec_ids),
                        fmt_codec(bh.codec_seq),
                        fmt_codec(bh.codec_qual),
                        fmt_codec(bh.codec_aux)
                    );
                }
            }
        }

        Ok(())
    }
}
