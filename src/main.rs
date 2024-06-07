mod tokenizer;

const CODE: &str = include_str!("../Cඞඞ.sus");

fn log10(n: usize) -> usize {
    (n as f64).log10().ceil() as usize
}

fn main() {
    let tokens = tokenizer::tokenize(CODE);

    let line_dwidth = log10(tokens.line_breaks.len());

    let mut col_dwidth = 0;
    let mut type_dwidth = 0;
    for (&ty, &(_, _, col)) in tokens.types.iter().zip(tokens.spans.iter()) {
        col_dwidth = col_dwidth.max(log10(col));
        type_dwidth = type_dwidth.max(format!("{ty:?}").len());
    }

    for (ty, (span_slice, line, col)) in tokens.types.iter().zip(tokens.spans.iter()) {
        println!(
            "{:>line_dwidth$}:{:<col_dwidth$}   {:<type_dwidth$}   {span_slice}",
            line,
            col,
            format!("{ty:?}"),
            line_dwidth = line_dwidth,
            col_dwidth = col_dwidth,
            type_dwidth = type_dwidth,
        );
    }
}
