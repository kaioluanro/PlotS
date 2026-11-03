//! Modo de teste de `imprimir`: conversa com o simulador pelo canal local. Só existe com `--teste`.
//!
//! Cada mensagem é uma linha em uma conexão própria. O simulador só responde ao gesto depois de
//! cumpri-lo, e por isso o verso nunca parte antes de o maço estar na entrada.

use plots_print::fluxo::{BordaQueEntra, EntradaDoMaco, FaceImpressa, Instrucao};
use std::fs::OpenOptions;
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Canal do simulador.
pub(crate) const CANAL_DO_SIMULADOR: &str = r"\\.\pipe\plotsim";
/// O modo de teste só envia a filas com este prefixo: nenhuma outra é virtual.
pub(crate) const PREFIXO_DA_FILA: &str = "PlotSim — ";
/// Maior resposta aceita, em bytes.
const LIMITE_RESPOSTA: usize = 512;
/// Quanto esperar por um lugar no canal.
const PRAZO_PARA_CONECTAR: Duration = Duration::from_secs(10);
/// Quanto esperar pela resposta: o simulador pode estar lendo o trabalho da frente.
const PRAZO_DA_RESPOSTA: Duration = Duration::from_secs(300);
const PAUSA_ENTRE_TENTATIVAS: Duration = Duration::from_millis(50);
/// Erro do Windows quando todas as instâncias do canal estão ocupadas.
const ERRO_CANAL_OCUPADO: i32 = 231;
const ERRO_CANAL_NAO_EXISTE: i32 = 2;

/// Como a pessoa pega o maço para ler, se o programa disser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LeituraDoMaco {
    ComoSaiu,
    Virando,
    SemInstrucao,
}

fn codigo_da_entrada(entrada: EntradaDoMaco) -> &'static str {
    match entrada {
        EntradaDoMaco::Traseira => "traseira",
        EntradaDoMaco::Gaveta => "gaveta",
        EntradaDoMaco::Manual => "manual",
    }
}

fn codigo_da_face(face: FaceImpressa) -> &'static str {
    match face {
        FaceImpressa::ParaFrente => "para_frente",
        FaceImpressa::ParaTras => "para_tras",
        FaceImpressa::ParaCima => "para_cima",
        FaceImpressa::ParaBaixo => "para_baixo",
    }
}

fn codigo_da_borda(borda: BordaQueEntra) -> &'static str {
    match borda {
        BordaQueEntra::Topo => "topo",
        BordaQueEntra::Base => "base",
    }
}

pub(crate) fn linha_de_inicio(fila: &str, lotes: usize) -> String {
    format!("plotsim\t1\tinicio\tfila={fila}\tlotes={lotes}\n")
}

pub(crate) fn linha_do_gesto(fila: &str, lote: usize, instrucao: &Instrucao) -> String {
    format!(
        "plotsim\t1\tgesto\tfila={fila}\tlote={lote}\tentrada={}\tface_impressa={}\tborda_que_entra={}\tinverter_ordem={}\n",
        codigo_da_entrada(instrucao.entrada),
        codigo_da_face(instrucao.face_impressa),
        codigo_da_borda(instrucao.borda_que_entra),
        if instrucao.inverter_ordem { "sim" } else { "nao" },
    )
}

pub(crate) fn linha_de_fim(fila: &str, lotes: usize, leitura: LeituraDoMaco) -> String {
    let leitura = match leitura {
        LeituraDoMaco::ComoSaiu => "como_saiu",
        LeituraDoMaco::Virando => "virando",
        LeituraDoMaco::SemInstrucao => "sem_instrucao",
    };
    format!("plotsim\t1\tfim\tfila={fila}\tlotes={lotes}\tleitura={leitura}\n")
}

/// O que o simulador respondeu. Qualquer coisa fora do combinado é recusa.
fn interpretar_resposta(bytes: &[u8]) -> Result<(), String> {
    let texto = String::from_utf8_lossy(bytes);
    let linha = texto.trim_end_matches(['\r', '\n']);
    if linha == "ok" {
        return Ok(());
    }
    match linha.strip_prefix("erro\t") {
        Some(motivo) => Err(format!("o simulador recusou: {motivo}")),
        None => Err("o simulador respondeu algo que não dá para entender".to_string()),
    }
}

/// Canal por onde o modo de teste publica. O caminho só muda nos testes.
pub(crate) struct Canal {
    caminho: PathBuf,
}

impl Canal {
    pub(crate) fn do_simulador() -> Self {
        Self::em(PathBuf::from(CANAL_DO_SIMULADOR))
    }

    pub(crate) fn em(caminho: PathBuf) -> Self {
        Self { caminho }
    }

    /// Manda uma linha e espera a resposta. Nunca espera sem prazo.
    pub(crate) fn publicar(&self, linha: &str) -> Result<(), String> {
        let inicio = Instant::now();
        let mut canal = loop {
            match OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.caminho)
            {
                Ok(canal) => break canal,
                Err(erro)
                    if erro.raw_os_error() == Some(ERRO_CANAL_OCUPADO)
                        && inicio.elapsed() < PRAZO_PARA_CONECTAR =>
                {
                    std::thread::sleep(PAUSA_ENTRE_TENTATIVAS);
                }
                Err(erro) => return Err(self.sem_canal(&erro)),
            }
        };
        canal
            .write_all(linha.as_bytes())
            .map_err(|erro| format!("não deu para escrever no canal do simulador: {erro}"))?;
        canal
            .flush()
            .map_err(|erro| format!("não deu para escrever no canal do simulador: {erro}"))?;
        let (destino, chegada) = mpsc::channel();
        // A leitura bloqueia. Se o prazo passar, a thread acaba com o processo.
        std::thread::spawn(move || {
            let _ = destino.send(ler_uma_linha(&mut canal));
        });
        let bytes = chegada
            .recv_timeout(PRAZO_DA_RESPOSTA)
            .map_err(|_| "o simulador não respondeu a tempo".to_string())?
            .map_err(|erro| format!("o canal do simulador caiu: {erro}"))?;
        interpretar_resposta(&bytes)
    }

    fn sem_canal(&self, erro: &std::io::Error) -> String {
        let ausente = erro.kind() == ErrorKind::NotFound
            || erro.raw_os_error() == Some(ERRO_CANAL_NAO_EXISTE);
        if ausente {
            format!(
                "o canal do simulador {} não está aberto; --teste só funciona com o PlotSim rodando, e nada foi enviado à fila por causa disso",
                self.caminho.display()
            )
        } else {
            format!(
                "não deu para abrir o canal do simulador {}: {erro}",
                self.caminho.display()
            )
        }
    }
}

/// Lê até o fim da linha, sem passar do limite.
fn ler_uma_linha(canal: &mut impl Read) -> std::io::Result<Vec<u8>> {
    let mut lidos = Vec::new();
    let mut pedaco = [0u8; 128];
    loop {
        let n = canal.read(&mut pedaco)?;
        let Some(novos) = pedaco.get(..n) else {
            return Ok(lidos);
        };
        if novos.is_empty() {
            return Ok(lidos);
        }
        lidos.extend_from_slice(novos);
        if lidos.contains(&b'\n') || lidos.len() >= LIMITE_RESPOSTA {
            lidos.truncate(LIMITE_RESPOSTA);
            return Ok(lidos);
        }
    }
}

/// Lê `I/N` de `--parte`.
pub(crate) fn analisar_parte(valor: &str) -> Result<(usize, usize), String> {
    const ERRO: &str = "--parte deve ser I/N, com I de 1 até N";
    let (indice, total) = valor.split_once('/').ok_or(ERRO)?;
    let indice: usize = indice.parse().map_err(|_| ERRO)?;
    let total: usize = total.parse().map_err(|_| ERRO)?;
    if indice == 0 || indice > total {
        return Err(ERRO.to_string());
    }
    Ok((indice, total))
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_core::ModeloImpressora;
    use plots_print::fluxo::Virada;
    use std::io::Cursor;

    const FILA: &str = "PlotSim — Jato";

    #[test]
    fn as_linhas_levam_os_quatro_campos_e_o_lote() {
        assert_eq!(
            linha_do_gesto(FILA, 3, &Instrucao::do_modelo(ModeloImpressora::JatoDeTinta)),
            "plotsim\t1\tgesto\tfila=PlotSim — Jato\tlote=3\tentrada=traseira\tface_impressa=para_tras\tborda_que_entra=base\tinverter_ordem=nao\n"
        );
        assert_eq!(
            linha_do_gesto(FILA, 1, &Instrucao::do_modelo(ModeloImpressora::Laser)),
            "plotsim\t1\tgesto\tfila=PlotSim — Jato\tlote=1\tentrada=gaveta\tface_impressa=para_baixo\tborda_que_entra=base\tinverter_ordem=nao\n"
        );
        assert_eq!(
            linha_de_inicio(FILA, 2),
            "plotsim\t1\tinicio\tfila=PlotSim — Jato\tlotes=2\n"
        );
        assert_eq!(
            linha_de_fim(FILA, 2, LeituraDoMaco::Virando),
            "plotsim\t1\tfim\tfila=PlotSim — Jato\tlotes=2\tleitura=virando\n"
        );
        assert_eq!(
            Instrucao::do_modelo(ModeloImpressora::Laser).virada,
            Virada::NaoVirar
        );
    }

    #[test]
    fn resposta_fora_do_combinado_e_recusa() {
        assert_eq!(interpretar_resposta(b"ok\n"), Ok(()));
        assert_eq!(interpretar_resposta(b"ok\r\n"), Ok(()));
        assert_eq!(
            interpretar_resposta(b"erro\tlote fora de ordem\n"),
            Err("o simulador recusou: lote fora de ordem".to_string())
        );
        for lixo in [&b""[..], b"okay\n", b"\xff\xfe", b"erro sem tab\n"] {
            assert!(interpretar_resposta(lixo).is_err(), "{lixo:?}");
        }
    }

    #[test]
    fn resposta_sem_fim_nao_passa_do_limite() {
        let mut enorme = Cursor::new(vec![b'x'; 100_000]);
        assert_eq!(ler_uma_linha(&mut enorme).unwrap().len(), LIMITE_RESPOSTA);
        let mut curta = Cursor::new(b"ok\nresto".to_vec());
        assert_eq!(ler_uma_linha(&mut curta).unwrap(), b"ok\nresto");
        let mut vazia = Cursor::new(Vec::new());
        assert!(ler_uma_linha(&mut vazia).unwrap().is_empty());
    }

    #[test]
    fn sem_o_canal_a_falha_diz_o_que_falta() {
        let canal = Canal::em(PathBuf::from(r"\\.\pipe\plotsim-canal-que-nao-existe"));
        let erro = canal.publicar("oi\n").unwrap_err();
        assert!(erro.contains("não está aberto"), "{erro}");
        assert!(erro.contains("--teste"), "{erro}");
    }

    #[test]
    fn parte_so_aceita_indice_dentro_do_total() {
        assert_eq!(analisar_parte("1/2"), Ok((1, 2)));
        assert_eq!(analisar_parte("2/2"), Ok((2, 2)));
        for ruim in [
            "0/2",
            "3/2",
            "1",
            "a/b",
            "1/0",
            "/",
            "-1/2",
            "99999999999999999999/1",
        ] {
            assert!(analisar_parte(ruim).is_err(), "{ruim}");
        }
    }
}
