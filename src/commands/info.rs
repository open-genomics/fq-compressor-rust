// =============================================================================
// fqc-rust - Info Command
// =============================================================================

use crate::archive::reader::{ArchiveInfo, FqcReader};
use crate::error::Result;
use crate::println_stdout;
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
        println_stdout!("{{");
        println_stdout!("  \"file\": \"{}\",", info.file_path);
        println_stdout!("  \"file_size\": {},", info.file_size);
        println_stdout!("  \"total_reads\": {},", info.total_reads);
        println_stdout!("  \"num_blocks\": {},", info.num_blocks);
        println_stdout!("  \"original_filename\": \"{}\",", info.original_filename);
        println_stdout!("  \"timestamp\": {},", info.timestamp);
        println_stdout!("  \"is_paired\": {},", info.is_paired);
        println_stdout!("  \"has_reorder_map\": {},", info.has_reorder_map);
        println_stdout!("  \"preserve_order\": {},", info.preserve_order);
        println_stdout!("  \"streaming_mode\": {},", info.streaming_mode);
        println_stdout!("  \"quality_mode\": \"{}\",", info.quality_mode.as_str());
        println_stdout!("  \"id_mode\": \"{}\",", info.id_mode.as_str());
        println_stdout!("  \"pe_layout\": \"{}\",", info.pe_layout.as_str());
        println_stdout!("  \"read_length_class\": \"{}\"", info.read_length_class.as_str());
        println_stdout!("}}");
    }

    fn print_human(&self, info: &ArchiveInfo, reader: &mut FqcReader) -> Result<()> {
        println_stdout!("File:              {}", info.file_path);
        println_stdout!("File size:         {} bytes", info.file_size);
        println_stdout!("Total reads:       {}", info.total_reads);
        println_stdout!("Num blocks:        {}", info.num_blocks);
        println_stdout!("Original filename: {}", info.original_filename);
        println_stdout!("Is paired-end:     {}", info.is_paired);
        println_stdout!("Has reorder map:   {}", info.has_reorder_map);
        println_stdout!("Preserve order:    {}", info.preserve_order);
        println_stdout!("Streaming mode:    {}", info.streaming_mode);
        println_stdout!("Quality mode:      {}", info.quality_mode.as_str());
        println_stdout!("ID mode:           {}", info.id_mode.as_str());
        println_stdout!("PE layout:         {}", info.pe_layout.as_str());
        println_stdout!("Read length class: {}", info.read_length_class.as_str());

        if self.opts.detailed {
            println_stdout!("\nBlock Index:");
            println_stdout!(
                "  {:>6}  {:>12}  {:>12}  {:>10}  {:>10}",
                "Block",
                "Offset",
                "CompSize",
                "ArchiveID",
                "Reads"
            );
            for (i, entry) in reader.block_index.entries.iter().enumerate() {
                println_stdout!(
                    "  {:>6}  {:>12}  {:>12}  {:>10}  {:>10}",
                    i,
                    entry.offset,
                    entry.compressed_size,
                    entry.archive_id_start,
                    entry.read_count
                );
            }
        }

        if self.opts.show_codecs {
            let num_blocks = reader.block_count();
            println_stdout!("\nBlock Codecs:");
            println_stdout!(
                "  {:>6}  {:>12}  {:>12}  {:>12}  {:>12}",
                "Block",
                "IDs",
                "Seq",
                "Qual",
                "Aux"
            );
            for i in 0..num_blocks {
                if let Ok(bh) = reader.read_block_header(i as u32) {
                    let fmt_codec = |c: u8| -> String {
                        let family = decode_codec_family(c);
                        let version = c & 0x0F;
                        format!("{:?}v{}", family, version)
                    };
                    println_stdout!(
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
