use plots_cli::executar_com_io;

fn main() {
    let argumentos: Vec<String> = std::env::args().skip(1).collect();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    if let Err(erro) = executar_com_io(&argumentos, &mut stdin.lock(), &mut stdout.lock()) {
        eprintln!("Erro: {erro}\n\nUso: plots simular --paginas N --lote K --impressora traseira|gaveta [--guiado]");
        std::process::exit(2);
    }
}
