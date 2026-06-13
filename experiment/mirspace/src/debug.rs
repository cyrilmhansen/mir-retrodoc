use crate::space::ProgramSpace;

impl ProgramSpace {
    pub fn debug_summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Module: {}\n", self.name));
        s.push_str(&format!("Functions: {}\n", self.functions.len()));
        s.push_str(&format!("Blocks: {}\n", self.blocks.len()));
        s.push_str(&format!("Instructions: {}\n", self.instructions.len()));
        s.push_str(&format!("Edges: {}\n", self.edges.len()));
        s.push_str(&format!("Data Segments: {}\n", self.data_segments.len()));

        for func in &self.functions {
            let func_name = self
                .symbols
                .get(self.maps.symbols[&func.symbol].0)
                .map(|sym| sym.name.as_str())
                .unwrap_or("unknown");

            let mut inst_count = 0;
            for &block_ix in &func.blocks {
                let block = &self.blocks[block_ix.0];
                inst_count += block.instructions.len();
            }

            let mut edge_count = 0;
            for edge in &self.edges {
                let source_block = &self.blocks[edge.source.0];
                if source_block.parent.0 == self.maps.functions[&func.id].0 {
                    edge_count += 1;
                }
            }

            s.push_str(&format!(
                "Function {} ({}):\n  Blocks: {}\n  Instructions: {}\n  Edges: {}\n",
                func.id.0,
                func_name,
                func.blocks.len(),
                inst_count,
                edge_count
            ));
        }
        s
    }
}
