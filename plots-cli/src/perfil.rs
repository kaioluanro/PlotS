//! Comando `perfil`: grava o perfil pronto de um modelo, sem medir a impressora.

use crate::calibrar::{
    caminho_perfis, carregar_perfis, descrever_perfil, msg_impressora_simulada, salvar_perfis,
};
use crate::{nome_do_modelo, ImpressoraVirtual};
use plots_core::ModeloImpressora;
use std::io::Write;
use std::path::PathBuf;

/// Executa `perfil NOME --modelo jato|laser [--perfis ARQUIVO]`.
pub(crate) fn executar(args: &[String], saida: &mut impl Write) -> Result<(), String> {
    let fila = args
        .get(1)
        .filter(|s| !s.starts_with("--") && !s.trim().is_empty())
        .ok_or("informe o nome da impressora")?;
    let (mut modelo, mut perfis) = (None, None);
    let mut i = 2;
    while i < args.len() {
        let chave = &args[i];
        let valor = args
            .get(i + 1)
            .ok_or_else(|| format!("faltou valor para {chave}"))?;
        match chave.as_str() {
            "--modelo" if modelo.is_none() => {
                modelo = Some(
                    ModeloImpressora::de_codigo(valor).ok_or("--modelo deve ser jato ou laser")?,
                )
            }
            "--perfis" if perfis.is_none() => perfis = Some(PathBuf::from(valor)),
            _ => return Err(format!("opção desconhecida ou repetida: {chave}")),
        }
        i += 2;
    }
    let modelo = modelo.ok_or("informe --modelo jato|laser")?;
    if ImpressoraVirtual::de_nome(fila).is_some() {
        return Err(msg_impressora_simulada(fila));
    }
    let caminho = caminho_perfis(perfis)?;
    let mut lidos = carregar_perfis(&caminho)?;
    lidos.definir_modelo(fila, modelo);
    salvar_perfis(&caminho, &lidos)?;
    writeln!(
        saida,
        "Perfil de {fila} gravado em {}: modelo {}.",
        caminho.display(),
        nome_do_modelo(modelo)
    )
    .map_err(|e| e.to_string())?;
    for linha in descrever_perfil(&modelo.perfil()) {
        writeln!(saida, "  {linha}").map_err(|e| e.to_string())?;
    }
    writeln!(
        saida,
        "O modelo não foi medido nesta máquina. É um ponto de partida; confira com 2 ou 3 folhas."
    )
    .map_err(|e| e.to_string())
}
