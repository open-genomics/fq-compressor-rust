// =============================================================================
// fqc-rust - Info Command
// =============================================================================

use crate::error::Result;
use crate::format::{get_id_mode, get_pe_layout, get_quality_mode, get_read_length_class};
use crate::fqc_reader::FqcReader;

// =============================================================================
// InfoOptions
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct InfoOptions {
    pub input_path: String,
    pub json: bool,
    pub detailed: bool,
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
                1
            }
        }
    }

    fn run(&self) -> Result<()> {
        let reader = FqcReader::open(&self.opts.input_path)?;

        let flags = reader.global_header.flags;
        let quality_mode = get_quality_mode(flags);
        let id_mode = get_id_mode(flags);
        let pe_layout = get_pe_layout(flags);
        let length_class = get_read_length_class(flags);

        let is_paired = (flags & crate::format::flags::IS_PAIRED) != 0;
        let has_reorder_map = (flags & crate::format::flags::HAS_REORDER_MAP) != 0;
        let preserve_order = (flags & crate::format::flags::PRESERVE_ORDER) != 0;
        let streaming_mode = (flags & crate::format::flags::STREAMING_MODE) != 0;

        if self.opts.json {
            println!("{{");
            println!("  \"file\": \"{}\",", self.opts.input_path);
            println!("  \"file_size\": {},", reader.file_size);
            println!("  \"total_reads\": {},", reader.global_header.total_read_count);
            println!("  \"num_blocks\": {},", reader.block_count());
            println!("  \"original_filename\": \"{}\",", reader.global_header.original_filename);
            println!("  \"timestamp\": {},", reader.global_header.timestamp);
            println!("  \"is_paired\": {},", is_paired);
            println!("  \"has_reorder_map\": {},", has_reorder_map);
            println!("  \"preserve_order\": {},", preserve_order);
            println!("  \"streaming_mode\": {},", streaming_mode);
            println!("  \"quality_mode\": \"{}\",", quality_mode.as_str());
            println!("  \"id_mode\": \"{}\",", id_mode.as_str());
            println!("  \"pe_layout\": \"{}\",", pe_layout.as_str());
            println!("  \"read_length_class\": \"{}\"", length_class.as_str());
            println!("}}");
        } else {
            println!("File:              {}", self.opts.input_path);
            println!("File size:         {} bytes", reader.file_size);
            println!("Total reads:       {}", reader.global_header.total_read_count);
            println!("Num blocks:        {}", reader.block_count());
            println!("Original filename: {}", reader.global_header.original_filename);
            println!("Is paired-end:     {}", is_paired);
            println!("Has reorder map:   {}", has_reorder_map);
            println!("Preserve order:    {}", preserve_order);
            println!("Streaming mode:    {}", streaming_mode);
            println!("Quality mode:      {}", quality_mode.as_str());
            println!("ID mode:           {}", id_mode.as_str());
            println!("PE layout:         {}", pe_layout.as_str());
            println!("Read length class: {}", length_class.as_str());

            if self.opts.detailed {
                println!("\nBlock Index:");
                println!("  {:>6}  {:>12}  {:>12}  {:>10}  {:>10}",
                    "Block", "Offset", "CompSize", "ArchiveID", "Reads");
                for (i, entry) in reader.block_index.entries.iter().enumerate() {
                    println!("  {:>6}  {:>12}  {:>12}  {:>10}  {:>10}",
                        i, entry.offset, entry.compressed_size,
                        entry.archive_id_start, entry.read_count);
                }
            }
        }

        Ok(())
    }
}
