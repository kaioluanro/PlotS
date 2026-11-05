//! O estado do programa: as filas, a bandeja de cada uma e quem acompanha o spooler.

use crate::bandeja::{Aviso, Bandeja, BandejaTela, RecusaDoGesto};
use crate::erros::{aviso_de_leitura, aviso_de_recepcao, da_recepcao, das_filas, ErroTela};
use crate::marca::ler_pagina;
use crate::ouvinte::{self, Novidade, Ouvinte};
use crate::plataforma;
use plotsim_core::canal::LeituraPedida;
use plotsim_core::recepcao::{Evento, Recepcao};
use plotsim_core::{
    ErroPasta, FaceDeSaida, FilaVirtual, Filas, ModoFisico, NomeVirtual, Pastas, Saida, Situacao,
};
use plotsim_physics::{Gesto, Idioma};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Modo da fila como a tela o recebe.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModoTela {
    Jato,
    Laser,
    TraseiraFaceParaBaixo,
    GavetaFaceParaCima,
    SemModo,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EstadoFila {
    Ociosa,
    Recebendo,
    AguardandoVerso,
    VersoRecebido,
    ComAviso,
    SemModo,
    ForaDoWindows,
    Diferente,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FilaTela {
    pub nome: String,
    pub modo: ModoTela,
    pub estado: EstadoFila,
    pub historico: String,
    /// Arquivo que o Windows grava com o último trabalho; outras contas do computador podem lê-lo.
    pub porta: String,
}

fn modo_da_tela(modo: Option<ModoFisico>) -> ModoTela {
    match modo.map(|m| (m.saida, m.face)) {
        None => ModoTela::SemModo,
        Some((Saida::Traseira, FaceDeSaida::ParaCima)) => ModoTela::Jato,
        Some((Saida::Gaveta, FaceDeSaida::ParaBaixo)) => ModoTela::Laser,
        Some((Saida::Traseira, FaceDeSaida::ParaBaixo)) => ModoTela::TraseiraFaceParaBaixo,
        Some((Saida::Gaveta, FaceDeSaida::ParaCima)) => ModoTela::GavetaFaceParaCima,
    }
}

/// O modo pedido na janela de nova impressora. Só os dois da especificação são oferecidos.
pub fn modo_pedido(jato: bool) -> ModoFisico {
    if jato {
        ModoFisico {
            saida: Saida::Traseira,
            face: FaceDeSaida::ParaCima,
        }
    } else {
        ModoFisico {
            saida: Saida::Gaveta,
            face: FaceDeSaida::ParaBaixo,
        }
    }
}

fn chave(nome: &str) -> String {
    nome.to_lowercase()
}

fn travar<T>(trava: &Mutex<T>) -> MutexGuard<'_, T> {
    trava.lock().unwrap_or_else(PoisonError::into_inner)
}

struct NaMemoria {
    modo: ModoFisico,
    bandeja: Bandeja,
    historico: PathBuf,
}

/// Compartilhado com as linhas dos ouvintes.
#[derive(Default)]
struct Compartilhado {
    bandejas: Mutex<HashMap<String, NaMemoria>>,
    ouvintes: Mutex<HashMap<String, Ouvinte>>,
}

pub struct Estado {
    pastas: Result<Pastas, ErroPasta>,
    compartilhado: Arc<Compartilhado>,
    avisar: Arc<dyn Fn() + Send + Sync>,
    /// `Some` quando o canal com o PlotS não abriu: a tela avisa, e o gesto não chega.
    canal_erro: Mutex<Option<String>>,
}

/// Quanto a parada de uma linha espera por ela. Passou disso, a linha segue presa no sistema,
/// sem prender a tela.
const PRAZO_PARA_PARAR: Duration = Duration::from_secs(2);

/// Apaga o último PDF que a porta da fila guarda. Ausente já é o pedido cumprido; se o Windows
/// recusar (por exemplo, o spooler ainda o usa), o motivo volta para a tela.
fn apagar_arquivo_da_porta(porta: &Path) -> Result<(), ErroTela> {
    match std::fs::remove_file(porta) {
        Ok(()) => Ok(()),
        Err(erro) if erro.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(erro) => Err(ErroTela::com("porta_presa", erro.to_string())),
    }
}

impl Estado {
    /// `avisar` é chamado quando uma bandeja muda sozinha, para a tela olhar de novo.
    pub fn novo(pastas: Result<Pastas, ErroPasta>, avisar: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            pastas,
            compartilhado: Arc::default(),
            avisar,
            canal_erro: Mutex::new(None),
        }
    }

    /// Guarda se o canal com o PlotS abriu; a tela consulta isso.
    pub fn guardar_canal(&self, erro: Option<String>) {
        *travar(&self.canal_erro) = erro;
    }

    /// Se o canal com o PlotS não abriu.
    pub fn canal_fechado(&self) -> bool {
        travar(&self.canal_erro).is_some()
    }

    /// Para a linha da fila e espera até o prazo. Feito antes de apagar arquivos, para que nada
    /// seja gravado depois da limpeza. Devolve se a linha parou; sem linha, já está parada.
    /// A próxima leitura das filas põe a linha de volta.
    fn parar_ouvinte(&self, curto: &str) -> bool {
        let ouvinte = travar(&self.compartilhado.ouvintes).remove(&chave(curto));
        ouvinte.is_none_or(|ouvinte| ouvinte.parar(PRAZO_PARA_PARAR))
    }

    /// O PlotS terminou o envio desta fila: a bandeja passa a conferir o maço da forma pedida.
    pub fn encerrar_fila(&self, curto: &str, leitura: LeituraPedida) {
        let encerrada = travar(&self.compartilhado.bandejas)
            .get_mut(&chave(curto))
            .map(|memoria| memoria.bandeja.encerrar(leitura))
            .is_some();
        if encerrada {
            (self.avisar)();
        }
    }

    fn pastas(&self) -> Result<&Pastas, ErroTela> {
        self.pastas
            .as_ref()
            .map_err(|erro| ErroTela::com("pasta", erro.to_string()))
    }

    fn filas(&self) -> Result<Filas<impl plotsim_core::SistemaDeFilas>, ErroTela> {
        Ok(Filas::novo(
            plataforma::sistema_de_filas(),
            self.pastas()?.clone(),
        ))
    }

    /// Lê as filas do sistema, que pode demorar, e acerta a memória com o resultado.
    pub fn listar(&self) -> Result<Vec<FilaTela>, ErroTela> {
        let filas = self.filas()?.listar().map_err(|erro| das_filas(&erro))?;
        let mut telas = Vec::with_capacity(filas.len());
        let mut vistas = Vec::with_capacity(filas.len());
        for fila in &filas {
            vistas.push(chave(fila.nome.curto()));
            self.preparar(fila);
            telas.push(self.fila_tela(fila)?);
        }
        self.esquecer_as_outras(&vistas);
        Ok(telas)
    }

    /// Fila que sumiu do sistema e do registro não tem mais quem a acompanhe.
    fn esquecer_as_outras(&self, vistas: &[String]) {
        travar(&self.compartilhado.ouvintes).retain(|nome, _| vistas.contains(nome));
        travar(&self.compartilhado.bandejas).retain(|nome, _| vistas.contains(nome));
    }

    fn fila_tela(&self, fila: &FilaVirtual) -> Result<FilaTela, ErroTela> {
        let historico = self.pastas()?.historico(&fila.nome);
        let bandejas = travar(&self.compartilhado.bandejas);
        let memoria = bandejas.get(&chave(fila.nome.curto()));
        let estado = match fila.situacao {
            Situacao::AusenteDoSistema => EstadoFila::ForaDoWindows,
            Situacao::SemRegistro => EstadoFila::SemModo,
            Situacao::Diferente => EstadoFila::Diferente,
            Situacao::Pronta => {
                memoria.map_or(EstadoFila::Ociosa, |m| estado_da_bandeja(&m.bandeja))
            }
        };
        Ok(FilaTela {
            nome: fila.nome.curto().to_string(),
            modo: modo_da_tela(fila.modo),
            estado,
            historico: historico.display().to_string(),
            porta: fila.porta.clone(),
        })
    }

    /// Garante bandeja e ouvinte para a fila pronta. Se o modo mudou, a bandeja antiga não vale mais.
    fn preparar(&self, fila: &FilaVirtual) {
        let nome = chave(fila.nome.curto());
        let (Situacao::Pronta, Some(modo)) = (fila.situacao, fila.modo) else {
            travar(&self.compartilhado.ouvintes).remove(&nome);
            travar(&self.compartilhado.bandejas).remove(&nome);
            return;
        };
        let Ok(pastas) = self.pastas() else {
            return;
        };
        let historico = pastas.historico(&fila.nome);
        {
            let mut bandejas = travar(&self.compartilhado.bandejas);
            let trocar = bandejas.get(&nome).is_none_or(|m| m.modo != modo);
            if trocar {
                bandejas.insert(
                    nome.clone(),
                    NaMemoria {
                        modo,
                        bandeja: Bandeja::nova(crate::bandeja::modo_do_registro(modo)),
                        historico: historico.clone(),
                    },
                );
            }
        }
        if travar(&self.compartilhado.ouvintes).contains_key(&nome) {
            return;
        }
        match self.iniciar_ouvinte(fila, historico, nome.clone()) {
            Ok(ouvinte) => {
                travar(&self.compartilhado.ouvintes).insert(nome, ouvinte);
            }
            Err(erro) => {
                if let Some(memoria) = travar(&self.compartilhado.bandejas).get_mut(&nome) {
                    memoria
                        .bandeja
                        .definir_aviso(Some(Aviso::novo("erro_de_fila", erro.detalhe.clone())));
                }
            }
        }
    }

    fn iniciar_ouvinte(
        &self,
        fila: &FilaVirtual,
        historico: PathBuf,
        nome: String,
    ) -> Result<Ouvinte, ErroTela> {
        let recepcao = Recepcao::nova(
            plataforma::spooler(),
            fila.nome.fila(),
            PathBuf::from(&fila.porta),
            historico,
        )
        .map_err(|erro| da_recepcao(&erro))?;
        let compartilhado = Arc::downgrade(&self.compartilhado);
        let avisar = Arc::clone(&self.avisar);
        ouvinte::iniciar(recepcao, move |novidade| {
            // O programa está fechando se o estado já se foi.
            if let Some(compartilhado) = compartilhado.upgrade() {
                ao_receber(&compartilhado, &nome, novidade);
                avisar();
            }
        })
        .map_err(|erro| ErroTela::com("erro_de_fila", erro.to_string()))
    }

    pub fn criar(&self, curto: &str, jato: bool) -> Result<Vec<FilaTela>, ErroTela> {
        self.filas()?
            .criar(curto, modo_pedido(jato))
            .map_err(|erro| das_filas(&erro))?;
        self.listar()
    }

    /// Remove a fila e a porta. O histórico só vai embora se a pessoa pediu.
    pub fn remover(&self, curto: &str, apagar_historico: bool) -> Result<Vec<FilaTela>, ErroTela> {
        let nome = NomeVirtual::novo(curto)
            .map_err(|erro| das_filas(&plotsim_core::ErroFilas::Nome(erro)))?;
        let filas = self.filas()?;
        // Primeiro para de olhar a fila, para não acusar erro no meio da remoção.
        if !self.parar_ouvinte(curto) {
            return Err(ErroTela::so("fila_ocupada"));
        }
        if let Err(erro) = filas.remover(curto) {
            // A fila continua lá; o próximo `listar` volta a acompanhá-la.
            let _ = self.listar();
            return Err(das_filas(&erro));
        }
        travar(&self.compartilhado.bandejas).remove(&chave(curto));
        if apagar_historico {
            let pasta = self.pastas()?.historico(&nome);
            crate::historico::limpar(&pasta)
                .map_err(|erro| ErroTela::com("historico", erro.to_string()))?;
        }
        self.listar()
    }

    pub fn bandeja(&self, curto: &str, idioma: Idioma) -> Result<BandejaTela, ErroTela> {
        travar(&self.compartilhado.bandejas)
            .get(&chave(curto))
            .map(|memoria| memoria.bandeja.tela(idioma))
            .ok_or_else(|| ErroTela::so("fila_desconhecida"))
    }

    /// Estado de cada bandeja conhecida, sem tocar no sistema.
    pub fn estados(&self) -> HashMap<String, EstadoFila> {
        travar(&self.compartilhado.bandejas)
            .iter()
            .map(|(nome, memoria)| (nome.clone(), estado_da_bandeja(&memoria.bandeja)))
            .collect()
    }

    /// Ponto de entrada do gesto escolhido pela pessoa na tela. O gesto do canal passa por
    /// `cumprir_gesto_do_canal`, que chama este mesmo caminho e avisa a tela.
    pub fn aplicar_gesto(&self, curto: &str, gesto: Gesto) -> Result<(), RecusaDoGesto> {
        let mut bandejas = travar(&self.compartilhado.bandejas);
        let memoria = bandejas
            .get_mut(&chave(curto))
            .ok_or(RecusaDoGesto::NadaParaRecolocar)?;
        memoria.bandeja.recolocar(gesto)
    }

    /// Gesto que chegou pelo canal do PlotS: cumpre na hora e avisa a tela, que pode estar olhando
    /// de longe; quem não pôs a mão no gesto não vê a mudança sem esse aviso.
    pub fn cumprir_gesto_do_canal(&self, curto: &str, gesto: Gesto) -> Result<(), RecusaDoGesto> {
        let cumprido = self.aplicar_gesto(curto, gesto);
        if cumprido.is_ok() {
            (self.avisar)();
        }
        cumprido
    }

    pub fn limpar_bandeja(&self, curto: &str) -> Result<(), ErroTela> {
        travar(&self.compartilhado.bandejas)
            .get_mut(&chave(curto))
            .map(|memoria| memoria.bandeja.limpar())
            .ok_or_else(|| ErroTela::so("fila_desconhecida"))
    }

    /// Quantos PDFs recebidos a fila guarda. Serve mesmo sem bandeja, como em fila fora do sistema.
    pub fn resumo_do_historico(&self, curto: &str) -> Result<(u64, u64), ErroTela> {
        let nome = NomeVirtual::novo(curto)
            .map_err(|erro| das_filas(&plotsim_core::ErroFilas::Nome(erro)))?;
        crate::historico::contar(&self.pastas()?.historico(&nome))
            .map_err(|erro| ErroTela::com("historico", erro.to_string()))
    }

    /// Apaga os PDFs recebidos e o último arquivo da porta. As folhas da tela dependem deles, então a
    /// bandeja é limpa junto. A linha da fila para antes, para não gravar PDF depois da limpeza.
    pub fn limpar_historico(&self, curto: &str) -> Result<u64, ErroTela> {
        let nome = NomeVirtual::novo(curto)
            .map_err(|erro| das_filas(&plotsim_core::ErroFilas::Nome(erro)))?;
        if !self.parar_ouvinte(curto) {
            return Err(ErroTela::so("fila_ocupada"));
        }
        let resultado = self.apagar_historico_e_porta(curto, &nome);
        // A linha da fila volta logo, com ou sem erro: sem isso, a fila deixa de receber até a
        // próxima leitura da lista.
        let _ = self.listar();
        resultado
    }

    fn apagar_historico_e_porta(&self, curto: &str, nome: &NomeVirtual) -> Result<u64, ErroTela> {
        let pastas = self.pastas()?;
        // A porta vem primeiro: se o spooler a estiver usando, nada mais é apagado e o aviso diz isso.
        let porta = pastas
            .porta(&nome)
            .map_err(|erro| das_filas(&plotsim_core::ErroFilas::Pasta(erro)))?;
        apagar_arquivo_da_porta(Path::new(&porta))?;
        if let Some(memoria) = travar(&self.compartilhado.bandejas).get_mut(&chave(curto)) {
            memoria.bandeja.limpar();
        }
        crate::historico::limpar(&pastas.historico(&nome))
            .map_err(|erro| ErroTela::com("historico", erro.to_string()))
    }

    /// Arquivo e posição da página. O caminho nunca vem da tela.
    pub fn origem_da_pagina(&self, curto: &str, pagina: u32) -> Option<(PathBuf, usize)> {
        let bandejas = travar(&self.compartilhado.bandejas);
        let memoria = bandejas.get(&chave(curto))?;
        let (arquivo, indice) = memoria.bandeja.origem_da_pagina(pagina)?;
        arquivo
            .starts_with(&memoria.historico)
            .then_some((arquivo, indice))
    }
}

fn estado_da_bandeja(bandeja: &Bandeja) -> EstadoFila {
    if bandeja.recebendo() {
        EstadoFila::Recebendo
    } else if bandeja.aviso().is_some() {
        EstadoFila::ComAviso
    } else if bandeja.aguardando_verso() {
        if bandeja.verso_recebido() {
            EstadoFila::VersoRecebido
        } else {
            EstadoFila::AguardandoVerso
        }
    } else {
        EstadoFila::Ociosa
    }
}

fn com_bandeja(compartilhado: &Compartilhado, nome: &str, acao: impl FnOnce(&mut Bandeja)) {
    if let Some(memoria) = travar(&compartilhado.bandejas).get_mut(nome) {
        acao(&mut memoria.bandeja);
    }
}

fn ao_receber(compartilhado: &Compartilhado, nome: &str, novidade: Novidade) {
    match novidade {
        Novidade::Ocupada(ocupada) => {
            com_bandeja(compartilhado, nome, |b| b.definir_recebendo(ocupada));
        }
        Novidade::Falha(Some(texto)) => {
            com_bandeja(compartilhado, nome, |b| {
                b.definir_aviso(Some(aviso_de_recepcao(&texto)));
            });
        }
        Novidade::Falha(None) => {
            com_bandeja(compartilhado, nome, |b| {
                if b.aviso().is_some_and(|a| a.codigo == "erro_de_fila") {
                    b.definir_aviso(None);
                }
            });
        }
        Novidade::Evento(evento) => ao_receber_evento(compartilhado, nome, evento),
    }
}

/// O PlotS nomeia o trabalho com a face (`PlotS - lote 1 - verso`). Qualquer outro nome nao diz nada.
fn e_verso_do_plots(documento: &str) -> bool {
    documento.starts_with("PlotS - lote ") && documento.ends_with(" - verso")
}

/// O PlotS nomeia a frente do mesmo jeito (`PlotS - lote 1 - frente`).
fn e_frente_do_plots(documento: &str) -> bool {
    documento.starts_with("PlotS - lote ") && documento.ends_with(" - frente")
}

fn ao_receber_evento(compartilhado: &Compartilhado, nome: &str, evento: Evento) {
    match evento {
        Evento::Recebido(trabalho) => {
            // Um verso do PlotS sem as frentes na bandeja nao e uma frente nova: nao empilha.
            if e_verso_do_plots(&trabalho.documento) {
                let mut sem_frente = false;
                com_bandeja(compartilhado, nome, |b| {
                    sem_frente = !b.aguardando_verso();
                    if sem_frente {
                        b.definir_aviso(Some(Aviso::novo(
                            "verso_sem_frente",
                            Some(trabalho.documento.clone()),
                        )));
                    }
                });
                if sem_frente {
                    return;
                }
            }
            // Ler o PDF pode demorar, e fica fora da trava: a tela segue respondendo.
            match plataforma::ler_marcas(&trabalho.arquivo) {
                Ok(paginas) => {
                    let vistas = paginas
                        .iter()
                        .enumerate()
                        .map(|(indice, pagina)| ler_pagina(indice, pagina))
                        .collect();
                    com_bandeja(compartilhado, nome, |b| {
                        b.definir_aviso(None);
                        if e_frente_do_plots(&trabalho.documento) {
                            b.receber_frente(trabalho.arquivo.clone(), vistas);
                        } else {
                            b.receber(trabalho.arquivo.clone(), vistas);
                        }
                    });
                }
                Err(erro) => {
                    com_bandeja(compartilhado, nome, |b| {
                        b.definir_aviso(Some(aviso_de_leitura(&erro)));
                    });
                }
            }
        }
        Evento::Falhou { documento, .. } => {
            com_bandeja(compartilhado, nome, |b| {
                b.definir_aviso(Some(Aviso::novo("trabalho_falhou", Some(documento))));
            });
        }
        Evento::SemArquivo { documento, .. } => {
            com_bandeja(compartilhado, nome, |b| {
                b.definir_aviso(Some(Aviso::novo("trabalho_sem_arquivo", Some(documento))));
            });
        }
        Evento::ArquivoRecusado { .. } => {
            com_bandeja(compartilhado, nome, |b| {
                b.definir_aviso(Some(Aviso::novo("pdf_grande", None)));
            });
        }
        Evento::Aviso(texto) => {
            com_bandeja(compartilhado, nome, |b| {
                b.definir_aviso(Some(aviso_de_recepcao(&texto)));
            });
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::marca::PaginaVista;
    use plotsim_core::recepcao::TrabalhoRecebido;
    use plotsim_core::spooler::{ErroSpooler, Spooler, TrabalhoNaFila};
    use plotsim_physics::{BordaQueEntra, Entrada, FaceNaEntrada, Giro};
    use std::fs;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    fn compartilhado_com_jato() -> Compartilhado {
        let compartilhado = Compartilhado::default();
        let modo = modo_pedido(true);
        travar(&compartilhado.bandejas).insert(
            "jato".to_string(),
            NaMemoria {
                modo,
                bandeja: Bandeja::nova(crate::bandeja::modo_do_registro(modo)),
                historico: PathBuf::from("h"),
            },
        );
        compartilhado
    }

    fn estado_de(compartilhado: &Compartilhado) -> EstadoFila {
        estado_da_bandeja(&travar(&compartilhado.bandejas)["jato"].bandeja)
    }

    fn aviso_de(compartilhado: &Compartilhado) -> Option<Aviso> {
        travar(&compartilhado.bandejas)["jato"]
            .bandeja
            .aviso()
            .cloned()
    }

    #[test]
    fn modo_pedido_e_os_nomes_da_tela_batem() {
        assert_eq!(modo_da_tela(Some(modo_pedido(true))), ModoTela::Jato);
        assert_eq!(modo_da_tela(Some(modo_pedido(false))), ModoTela::Laser);
        assert_eq!(modo_da_tela(None), ModoTela::SemModo);
    }

    #[test]
    fn receber_e_estado_passam_de_ociosa_a_recebendo_e_de_volta() {
        let compartilhado = compartilhado_com_jato();
        assert_eq!(estado_de(&compartilhado), EstadoFila::Ociosa);
        ao_receber(&compartilhado, "jato", Novidade::Ocupada(true));
        assert_eq!(estado_de(&compartilhado), EstadoFila::Recebendo);
        ao_receber(&compartilhado, "jato", Novidade::Ocupada(false));
        assert_eq!(estado_de(&compartilhado), EstadoFila::Ociosa);
    }

    const GESTO_JATO: Gesto = Gesto {
        entrada: Entrada::Traseira,
        face_impressa: FaceNaEntrada::ParaTras,
        borda_que_entra: BordaQueEntra::Base,
        inverter_ordem: false,
    };

    fn pagina_de_teste(numero: u32) -> PaginaVista {
        PaginaVista {
            indice: 0,
            numero: Some(numero),
            branca: false,
            giro: Giro::Nenhum,
            largura_mm: 210.0,
            altura_mm: 297.0,
        }
    }

    #[test]
    fn a_lista_distingue_o_verso_que_chegou_do_que_ainda_falta() {
        let modo = crate::bandeja::modo_do_registro(modo_pedido(true));
        let mut bandeja = Bandeja::nova(modo);
        bandeja.receber("frente.pdf".into(), vec![pagina_de_teste(2)]);
        assert_eq!(
            estado_da_bandeja(&bandeja),
            EstadoFila::AguardandoVerso,
            "a frente saiu e a passada do verso ainda não chegou"
        );
        bandeja.receber("verso.pdf".into(), vec![pagina_de_teste(1)]);
        assert_eq!(
            estado_da_bandeja(&bandeja),
            EstadoFila::VersoRecebido,
            "a passada do verso está na espera do gesto"
        );
        bandeja.recolocar(GESTO_JATO).unwrap();
        assert_eq!(
            estado_da_bandeja(&bandeja),
            EstadoFila::Ociosa,
            "o lote inteiro foi impresso e conferido"
        );
    }

    #[test]
    fn o_gesto_aplicado_antes_do_verso_continua_aguardando_o_verso() {
        let modo = crate::bandeja::modo_do_registro(modo_pedido(true));
        let mut bandeja = Bandeja::nova(modo);
        bandeja.receber("frente.pdf".into(), vec![pagina_de_teste(2)]);
        bandeja.recolocar(GESTO_JATO).unwrap();
        assert_eq!(
            estado_da_bandeja(&bandeja),
            EstadoFila::AguardandoVerso,
            "a pilha já está na entrada e a segunda passada ainda não chegou"
        );
    }

    #[test]
    fn um_aviso_ativo_vence_a_lista_dizer_que_o_verso_chegou() {
        let modo = crate::bandeja::modo_do_registro(modo_pedido(true));
        let mut bandeja = Bandeja::nova(modo);
        bandeja.receber("frente.pdf".into(), vec![pagina_de_teste(2)]);
        bandeja.receber("verso.pdf".into(), vec![pagina_de_teste(1)]);
        assert_eq!(estado_da_bandeja(&bandeja), EstadoFila::VersoRecebido);
        bandeja.definir_aviso(Some(Aviso::novo("pdf_ilegivel", None)));
        assert_eq!(
            estado_da_bandeja(&bandeja),
            EstadoFila::ComAviso,
            "o aviso é o que a pessoa precisa ver primeiro"
        );
    }

    #[test]
    fn falha_do_spooler_aparece_e_some_quando_a_fila_volta() {
        let compartilhado = compartilhado_com_jato();
        ao_receber(
            &compartilhado,
            "jato",
            Novidade::Falha(Some("recusou".to_string())),
        );
        assert_eq!(estado_de(&compartilhado), EstadoFila::ComAviso);
        assert_eq!(
            aviso_de(&compartilhado).map(|a| a.codigo),
            Some("erro_de_fila")
        );
        ao_receber(&compartilhado, "jato", Novidade::Falha(None));
        assert_eq!(aviso_de(&compartilhado), None);
    }

    #[test]
    fn volta_da_fila_nao_apaga_aviso_de_outra_origem() {
        let compartilhado = compartilhado_com_jato();
        ao_receber(
            &compartilhado,
            "jato",
            Novidade::Evento(Evento::ArquivoRecusado { bytes: 9 }),
        );
        ao_receber(&compartilhado, "jato", Novidade::Falha(None));
        assert_eq!(
            aviso_de(&compartilhado).map(|a| a.codigo),
            Some("pdf_grande")
        );
    }

    #[test]
    fn trabalho_que_falhou_ou_sumiu_vira_aviso_com_o_documento() {
        let compartilhado = compartilhado_com_jato();
        ao_receber_evento(
            &compartilhado,
            "jato",
            Evento::Falhou {
                id: 1,
                documento: "a".to_string(),
            },
        );
        assert_eq!(
            aviso_de(&compartilhado),
            Some(Aviso::novo("trabalho_falhou", Some("a".to_string())))
        );
        ao_receber_evento(
            &compartilhado,
            "jato",
            Evento::SemArquivo {
                id: 2,
                documento: "b".to_string(),
            },
        );
        assert_eq!(
            aviso_de(&compartilhado).map(|a| a.codigo),
            Some("trabalho_sem_arquivo")
        );
    }

    fn recebido(documento: &str) -> Evento {
        Evento::Recebido(TrabalhoRecebido {
            id: Some(1),
            documento: documento.to_string(),
            arquivo: PathBuf::from("nao-existe-mesmo.pdf"),
            bytes: 0,
        })
    }

    #[test]
    fn verso_do_plots_sem_a_frente_na_bandeja_vira_aviso_e_nao_entra_na_pilha() {
        let compartilhado = compartilhado_com_jato();
        ao_receber_evento(&compartilhado, "jato", recebido("PlotS - lote 1 - verso"));
        assert_eq!(
            aviso_de(&compartilhado),
            Some(Aviso::novo(
                "verso_sem_frente",
                Some("PlotS - lote 1 - verso".to_string())
            ))
        );
        let trava = travar(&compartilhado.bandejas);
        let tela = trava["jato"].bandeja.tela(Idioma::PtBr);
        assert!(tela.folhas.is_empty());
        assert_eq!(tela.paginas_sem_folha, 0);
    }

    #[test]
    fn nome_sem_a_face_do_plots_nao_e_tratado_como_verso() {
        let compartilhado = compartilhado_com_jato();
        for nome in ["doc - verso", "Verso.pdf", "PlotS - lote 1 - frente"] {
            ao_receber_evento(&compartilhado, "jato", recebido(nome));
            assert_ne!(
                aviso_de(&compartilhado).map(|a| a.codigo),
                Some("verso_sem_frente"),
                "{nome}"
            );
        }
    }

    #[test]
    fn pdf_que_nao_abre_vira_aviso_e_nao_cria_folha() {
        let compartilhado = compartilhado_com_jato();
        ao_receber_evento(
            &compartilhado,
            "jato",
            Evento::Recebido(TrabalhoRecebido {
                id: Some(1),
                documento: "a".to_string(),
                arquivo: PathBuf::from("nao-existe-mesmo.pdf"),
                bytes: 0,
            }),
        );
        let trava = travar(&compartilhado.bandejas);
        let tela = trava["jato"].bandeja.tela(Idioma::PtBr);
        assert!(tela.folhas.is_empty());
        assert!(tela.aviso.is_some());
    }

    fn pasta_de_teste(nome: &str) -> PathBuf {
        let caminho =
            std::env::temp_dir().join(format!("plotsim-estado-{}-{nome}", std::process::id()));
        let _ = fs::remove_dir_all(&caminho);
        fs::create_dir_all(&caminho).unwrap();
        caminho
    }

    #[test]
    fn limpar_historico_leva_junto_o_ultimo_pdf_da_porta() {
        let pasta = pasta_de_teste("porta-some");
        let porta = pasta.join("saida.pdf");
        fs::write(&porta, b"%PDF-1.4").unwrap();
        apagar_arquivo_da_porta(&porta).unwrap();
        assert!(!porta.exists());
        let _ = fs::remove_dir_all(pasta);
    }

    #[test]
    fn porta_sem_arquivo_ja_esta_limpa() {
        let pasta = pasta_de_teste("porta-ausente");
        assert_eq!(apagar_arquivo_da_porta(&pasta.join("saida.pdf")), Ok(()));
        let _ = fs::remove_dir_all(pasta);
    }

    #[test]
    fn porta_que_nao_se_apaga_vira_erro_com_o_motivo() {
        let pasta = pasta_de_teste("porta-travada");
        let porta = pasta.join("saida.pdf");
        fs::create_dir(&porta).unwrap();
        let erro = apagar_arquivo_da_porta(&porta).unwrap_err();
        assert_eq!(erro.codigo, "porta_presa");
        assert!(erro.detalhe.is_some());
        let _ = fs::remove_dir_all(pasta);
    }

    #[cfg(windows)]
    #[test]
    fn porta_presa_por_outro_processo_vira_erro() {
        use std::os::windows::fs::OpenOptionsExt;
        let pasta = pasta_de_teste("porta-presa");
        let porta = pasta.join("saida.pdf");
        fs::write(&porta, b"%PDF-1.4").unwrap();
        let preso = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&porta)
            .unwrap();
        let erro = apagar_arquivo_da_porta(&porta).unwrap_err();
        assert_eq!(erro.codigo, "porta_presa");
        drop(preso);
        assert!(porta.exists());
        let _ = fs::remove_dir_all(pasta);
    }

    #[test]
    fn porta_presa_nao_apaga_o_historico_e_avisa() {
        let raiz = pasta_de_teste("limpar-porta-presa");
        let pastas = Pastas::novas(raiz.join("config"), raiz.join("portas")).unwrap();
        let nome = NomeVirtual::novo("Teste").unwrap();
        let historico = pastas.historico(&nome);
        fs::create_dir_all(&historico).unwrap();
        fs::write(historico.join("00000001.pdf"), b"%PDF-1.4").unwrap();
        // Uma pasta no lugar do arquivo da porta não se apaga como arquivo: é a mesma recusa do Windows.
        let porta = PathBuf::from(pastas.porta(&nome).unwrap());
        fs::create_dir_all(&porta).unwrap();
        let estado = Estado::novo(Ok(pastas), Arc::new(|| {}));
        let erro = estado.limpar_historico("Teste").unwrap_err();
        assert_eq!(erro.codigo, "porta_presa");
        assert!(erro.detalhe.is_some());
        assert!(historico.join("00000001.pdf").exists());
        let _ = fs::remove_dir_all(raiz);
    }

    /// Um spooler que demora mais que o prazo de parada. Marca quando entra na chamada.
    #[derive(Clone)]
    struct SpoolerTravado(Arc<AtomicBool>);

    impl Spooler for SpoolerTravado {
        fn trabalhos(&self, _: &str) -> Result<Vec<TrabalhoNaFila>, ErroSpooler> {
            self.0.store(true, Ordering::SeqCst);
            thread::sleep(Duration::from_secs(3));
            Ok(Vec::new())
        }
    }

    /// Põe no estado a linha da fila "Teste" presa no spooler: a parada não chega no prazo.
    fn com_linha_presa(estado: &Estado, raiz: &Path) {
        let entrou = Arc::new(AtomicBool::new(false));
        let recepcao = Recepcao::nova(
            SpoolerTravado(Arc::clone(&entrou)),
            "PlotSim — Linha presa".to_string(),
            raiz.join("saida-da-linha.pdf"),
            raiz.join("historico-da-linha"),
        )
        .unwrap();
        let linha = ouvinte::iniciar(recepcao, |_| {}).unwrap();
        let inicio = std::time::Instant::now();
        while !entrou.load(Ordering::SeqCst) {
            assert!(
                inicio.elapsed() < Duration::from_secs(5),
                "a linha não começou"
            );
            thread::sleep(Duration::from_millis(10));
        }
        travar(&estado.compartilhado.ouvintes).insert(chave("Teste"), linha);
    }

    #[test]
    fn limpar_historico_recusa_com_a_linha_presa_e_nada_some() {
        let raiz = pasta_de_teste("limpar-linha-presa");
        let pastas = Pastas::novas(raiz.join("config"), raiz.join("portas")).unwrap();
        let nome = NomeVirtual::novo("Teste").unwrap();
        let historico = pastas.historico(&nome);
        fs::create_dir_all(&historico).unwrap();
        fs::write(historico.join("00000001.pdf"), b"%PDF-1.4").unwrap();
        let porta = PathBuf::from(pastas.porta(&nome).unwrap());
        fs::create_dir_all(porta.parent().unwrap()).unwrap();
        fs::write(&porta, b"%PDF-1.4").unwrap();
        let estado = Estado::novo(Ok(pastas), Arc::new(|| {}));
        com_linha_presa(&estado, &raiz);
        let erro = estado.limpar_historico("Teste").unwrap_err();
        assert_eq!(erro.codigo, "fila_ocupada");
        assert!(porta.exists());
        assert!(historico.join("00000001.pdf").exists());
        let _ = fs::remove_dir_all(raiz);
    }

    #[test]
    fn remover_recusa_com_a_linha_presa_e_o_historico_fica() {
        let raiz = pasta_de_teste("remover-linha-presa");
        let pastas = Pastas::novas(raiz.join("config"), raiz.join("portas")).unwrap();
        let nome = NomeVirtual::novo("Teste").unwrap();
        let historico = pastas.historico(&nome);
        fs::create_dir_all(&historico).unwrap();
        fs::write(historico.join("00000001.pdf"), b"%PDF-1.4").unwrap();
        let estado = Estado::novo(Ok(pastas), Arc::new(|| {}));
        com_linha_presa(&estado, &raiz);
        let erro = estado.remover("Teste", true).unwrap_err();
        assert_eq!(erro.codigo, "fila_ocupada");
        assert!(historico.join("00000001.pdf").exists());
        let _ = fs::remove_dir_all(raiz);
    }

    #[test]
    fn canal_que_nao_abriu_fica_marcado_para_a_tela() {
        let estado = Estado::novo(Err(ErroPasta::Relativo), Arc::new(|| {}));
        assert!(!estado.canal_fechado());
        estado.guardar_canal(Some("ocupado".to_string()));
        assert!(estado.canal_fechado());
        estado.guardar_canal(None);
        assert!(!estado.canal_fechado());
    }

    #[test]
    fn novidade_de_fila_que_ja_saiu_e_ignorada() {
        let compartilhado = compartilhado_com_jato();
        ao_receber(&compartilhado, "outra", Novidade::Ocupada(true));
        assert_eq!(estado_de(&compartilhado), EstadoFila::Ociosa);
    }
}
