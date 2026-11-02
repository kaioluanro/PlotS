use std::io::Write;
use std::path::Path;
use std::process::exit;

fn main() {
    // O contexto do Tauri embute a pasta gerada pelo Vite durante a compilacao, e essa pasta nao
    // vai para o Git. Sem ela a macro aborta, o que derrubaria `cargo test --workspace` em um
    // checkout novo; por isso a pasta passa a existir mesmo antes do primeiro `npm run build`.
    let pasta_gerada = Path::new("../ui/dist");
    if let Err(erro) = std::fs::create_dir_all(pasta_gerada) {
        falhar(&format!("nao foi possivel criar ui/dist: {erro}"));
    }
    println!("cargo:rerun-if-changed=../ui/dist");

    if let Err(erro) = tauri_build::try_build(tauri_build::Attributes::new()) {
        falhar(&format!("falha ao preparar o app: {erro}"));
    }
}

fn falhar(mensagem: &str) -> ! {
    let _ = writeln!(std::io::stderr(), "{mensagem}");
    exit(1);
}
