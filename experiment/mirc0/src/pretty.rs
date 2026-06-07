pub fn pretty_print_c(code: &str) -> String {
    let mut out = String::new();
    let mut indent = 0;
    
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push('\n');
            continue;
        }

        // Adjust indent down for close braces before printing
        let close_braces = trimmed.chars().filter(|&c| c == '}').count();
        let open_braces = trimmed.chars().filter(|&c| c == '{').count();
        
        let new_indent = indent + open_braces as i32 - close_braces as i32;
        
        let mut current_indent = indent;
        if trimmed.starts_with('}') {
            current_indent = (indent - 1).max(0);
        }
        
        // If it's a label, indent it slightly less for readability
        let is_label = trimmed.ends_with(':') && !trimmed.starts_with("default:") && !trimmed.starts_with("public:");
        if is_label {
            current_indent = (current_indent - 1).max(0);
        }
        
        // Don't indent preprocessor directives
        if trimmed.starts_with('#') {
            current_indent = 0;
        }
        
        for _ in 0..current_indent {
            out.push_str("    ");
        }
        
        out.push_str(trimmed);
        out.push('\n');
        
        indent = new_indent.max(0);
    }
    
    out
}
