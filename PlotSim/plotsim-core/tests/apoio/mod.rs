//! Apoio dos testes contra o Windows de verdade.
#![allow(dead_code)]

use lopdf::{dictionary, Document, Object, Stream};
use plotsim_core::Pastas;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PERGUNTA: &str = "(s/n): ";

pub fn saidas() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join("plots-build")
        .join("plotsim-filas")
        .join("saidas")
}

/// A configuração fica fora da pasta do usuário; a porta, na pasta do sistema, como de verdade.
pub fn pastas() -> Pastas {
    let portas = PathBuf::from(std::env::var_os("ProgramData").unwrap()).join("PlotSim");
    Pastas::novas(saidas().join("config-real"), portas).unwrap()
}

/// Tamanhos de papel em pontos.
pub const A4: (i64, i64) = (595, 842);
pub const A5: (i64, i64) = (420, 595);
pub const CARTA: (i64, i64) = (612, 792);

pub fn mm(pontos: i64) -> f64 {
    pontos as f64 * 25.4 / 72.0
}

/// PDF A4 com o número de cada página em texto grande.
pub fn pdf_numerado(paginas: u32) -> Vec<u8> {
    pdf_com_formatos(&vec![A4; paginas as usize])
}

/// Uma página por formato, com o número em texto grande.
pub fn pdf_com_formatos(formatos: &[(i64, i64)]) -> Vec<u8> {
    pdf_com(formatos, 0, |numero| format!("Pagina {numero}"))
}

/// Páginas com a rotação indicada e o texto que `texto` devolver para cada número.
pub fn pdf_com(formatos: &[(i64, i64)], rotacao: i64, texto: impl Fn(usize) -> String) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let raiz = doc.new_object_id();
    let fonte = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"
    });
    let mut filhos = Vec::new();
    for (indice, (largura, altura)) in formatos.iter().enumerate() {
        let conteudo = format!("BT /F1 48 Tf 40 300 Td ({}) Tj ET", texto(indice + 1));
        let fluxo = doc.add_object(Stream::new(dictionary! {}, conteudo.into_bytes()));
        let pagina = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => raiz,
            "MediaBox" => vec![0.into(), 0.into(), (*largura).into(), (*altura).into()],
            "Rotate" => rotacao,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => fonte } },
            "Contents" => fluxo
        });
        filhos.push(Object::Reference(pagina));
    }
    doc.objects.insert(
        raiz,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Count" => filhos.len() as i64, "Kids" => filhos
        }),
    );
    let catalogo = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => raiz});
    doc.trailer.set("Root", catalogo);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

/// O `plots imprimir` de verdade, respondendo às perguntas conforme elas aparecem.
pub struct EnvioPeloPlots {
    filho: Child,
    entrada: Option<ChildStdin>,
    saida: Arc<Mutex<Vec<u8>>>,
    respondidas: usize,
}

fn programa_plots() -> PathBuf {
    let caminho = std::env::var_os("PLOTSIM_PLOTS_EXE")
        .map(PathBuf::from)
        .expect("indique o executável em PLOTSIM_PLOTS_EXE");
    assert!(caminho.is_file(), "{} não existe", caminho.display());
    caminho
}

fn rodar_plots(args: &[&std::ffi::OsStr]) {
    let saida = Command::new(programa_plots()).args(args).output().unwrap();
    assert!(
        saida.status.success(),
        "{}",
        String::from_utf8_lossy(&saida.stderr)
    );
}

impl EnvioPeloPlots {
    /// Grava um perfil temporário para a fila e começa o envio do documento.
    pub fn iniciar(fila: &str, documento: &Path, lote: u32, pasta: &Path) -> Self {
        let perfis = pasta.join("perfis.toml");
        rodar_plots(&[
            "perfil".as_ref(),
            fila.as_ref(),
            "--modelo".as_ref(),
            "jato".as_ref(),
            "--perfis".as_ref(),
            perfis.as_os_str(),
        ]);
        let mut filho = Command::new(programa_plots())
            .arg("imprimir")
            .arg(documento)
            .args([
                "--impressora",
                fila,
                "--lote",
                &lote.to_string(),
                "--perfis",
            ])
            .arg(&perfis)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let entrada = filho.stdin.take();
        let saida = Arc::new(Mutex::new(Vec::new()));
        for fluxo in [
            filho
                .stdout
                .take()
                .map(|f| Box::new(f) as Box<dyn Read + Send>),
            filho
                .stderr
                .take()
                .map(|f| Box::new(f) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let destino = Arc::clone(&saida);
            std::thread::spawn(move || {
                let mut fluxo = fluxo;
                let mut pedaco = [0u8; 4096];
                while let Ok(lidos) = fluxo.read(&mut pedaco) {
                    if lidos == 0 {
                        break;
                    }
                    destino.lock().unwrap().extend_from_slice(&pedaco[..lidos]);
                }
            });
        }
        Self {
            filho,
            entrada,
            saida,
            respondidas: 0,
        }
    }

    fn texto(&self) -> String {
        String::from_utf8_lossy(&self.saida.lock().unwrap()).into_owned()
    }

    /// Espera a próxima pergunta. `false` quando o programa terminou sem perguntar mais.
    /// Passar do prazo é falha: é o que aconteceria com uma janela pedindo o nome do arquivo.
    pub fn esperar_pergunta(&mut self, prazo: Duration) -> bool {
        let inicio = Instant::now();
        loop {
            if self.texto().matches(PERGUNTA).count() > self.respondidas {
                return true;
            }
            if self.filho.try_wait().unwrap().is_some() {
                return false;
            }
            if inicio.elapsed() > prazo {
                let texto = self.texto();
                let _ = self.filho.kill();
                let _ = self.filho.wait();
                panic!("sem resposta em {prazo:?}; possível janela do driver. Saída:\n{texto}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn responder(&mut self, resposta: &str) {
        self.respondidas += 1;
        let entrada = self.entrada.as_mut().unwrap();
        writeln!(entrada, "{resposta}").unwrap();
        entrada.flush().unwrap();
    }

    /// Fecha a entrada, espera o fim e devolve tudo que o programa escreveu.
    pub fn terminar(mut self, prazo: Duration) -> String {
        drop(self.entrada.take());
        let inicio = Instant::now();
        loop {
            if let Some(status) = self.filho.try_wait().unwrap() {
                std::thread::sleep(Duration::from_millis(100));
                return format!("{}\nsaída do processo: {status}", self.texto());
            }
            if inicio.elapsed() > prazo {
                let texto = self.texto();
                let _ = self.filho.kill();
                let _ = self.filho.wait();
                panic!("o programa não terminou em {prazo:?}. Saída:\n{texto}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
