//! A execução de verdade: filas do Windows, `plots imprimir --teste` e o canal do simulador.
//!
//! Só toca em filas com o prefixo do simulador, e remove ao fim as que criou, mesmo com falha.

use crate::bancada::{conferir_maco, envio_da_leitura, erro_foi_acusado, juntar, Bancada, Etapa};
use crate::documento::{pdf_de_teste, Formato};
use crate::matriz::{Cenario, ModoDaFila, PerfilDaFila, Reparto};
use crate::relatorio::{Estado, ResultadoDoCenario};
use plotsim_core::canal::{
    ErroCanal, LeituraPedida, Mensagem, Pedido, ServidorDoCanal, CAMINHO_DO_CANAL,
};
use plotsim_core::filas::Filas;
use plotsim_core::leitor::{ler_marcas, PaginaMarcada};
use plotsim_core::powershell::{LeitorDoSpooler, PowerShell, ProcessoDoShell};
use plotsim_core::recepcao::{Evento, Observador, Recepcao, TrabalhoRecebido};
use plotsim_core::sistema::ErroSistema;
use plotsim_core::spooler::{Spooler, SpoolerDoWindows};
use plotsim_core::{ErroFilas, NomeVirtual, Pastas, SistemaDeFilas};
use plotsim_physics::{Envio, MacoLido, Motivo};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

type Sistema = PowerShell<ProcessoDoShell, LeitorDoSpooler>;

/// Prazo de um cenário: um tanto fixo, e um tanto por página. Passar disso é trabalho preso.
const PRAZO_BASE: Duration = Duration::from_secs(120);
const PRAZO_POR_PAGINA: Duration = Duration::from_secs(10);
/// Prazo de um comando curto do PlotS.
const PRAZO_DO_COMANDO: Duration = Duration::from_secs(30);
const INTERVALO: Duration = Duration::from_millis(20);
/// Quanto esperar por uma última mensagem depois de o PlotS sair.
const FOLGA_DEPOIS_DE_SAIR: Duration = Duration::from_secs(5);
/// Tolerância ao comparar o papel recebido com o do documento, em milímetros.
const TOLERANCIA_MM: f64 = 3.0;
const TENTATIVAS_DE_REMOVER: u32 = 5;
const PAUSA_ENTRE_REMOCOES: Duration = Duration::from_secs(2);

/// Falhas que impedem a suíte de seguir: a matriz para, e as filas são removidas.
#[derive(Debug)]
pub enum ErroDaSuite {
    /// O Windows negou. Sem elevação: a suíte para aqui.
    Negado(String),
    Canal(ErroCanal),
    Filas(String),
    Pasta(PathBuf, std::io::Error),
    Cancelada,
}

impl std::fmt::Display for ErroDaSuite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Negado(detalhe) => write!(
                f,
                "o Windows negou mexer nas filas e a suíte não pede elevação: {detalhe}"
            ),
            Self::Canal(erro) => erro.fmt(f),
            Self::Filas(detalhe) => write!(f, "não deu para preparar as filas: {detalhe}"),
            Self::Pasta(pasta, erro) => {
                write!(f, "não deu para usar {}: {erro}", pasta.display())
            }
            Self::Cancelada => f.write_str("a suíte foi interrompida"),
        }
    }
}

impl std::error::Error for ErroDaSuite {}

fn erro_de_filas(erro: ErroFilas) -> ErroDaSuite {
    match erro {
        ErroFilas::Sistema(ErroSistema::PrecisaAdministrador) => {
            ErroDaSuite::Negado(erro.to_string())
        }
        outro => ErroDaSuite::Filas(outro.to_string()),
    }
}

/// Uma fila criada pela suíte, sendo observada.
struct FilaAberta {
    curto: String,
    nome: String,
    eventos: Receiver<Evento>,
    observador: Option<Observador>,
}

/// Filas, canal e pastas da suíte. Ao sair de cena, remove tudo o que criou.
pub struct Ambiente {
    plots: PathBuf,
    trabalho: PathBuf,
    filas: Filas<Sistema>,
    abertas: HashMap<String, FilaAberta>,
    pedidos: Receiver<Pedido>,
    servidor: Option<ServidorDoCanal>,
    filas_antes: HashSet<String>,
    parar: &'static AtomicBool,
}

impl Ambiente {
    /// Abre o canal e guarda o que já existe no sistema. Nada ainda é criado.
    pub fn novo(
        plots: PathBuf,
        trabalho: PathBuf,
        portas: PathBuf,
        parar: &'static AtomicBool,
    ) -> Result<Self, ErroDaSuite> {
        fs::create_dir_all(&trabalho).map_err(|erro| ErroDaSuite::Pasta(trabalho.clone(), erro))?;
        let pastas = Pastas::novas(trabalho.join("config"), portas)
            .map_err(|erro| ErroDaSuite::Filas(erro.to_string()))?;
        let sistema = PowerShell::do_sistema();
        let filas_antes = sistema
            .filas()
            .map_err(|erro| ErroDaSuite::Filas(erro.to_string()))?
            .into_iter()
            .map(|fila| fila.nome)
            .collect();
        let (destino, pedidos) = mpsc::channel();
        let servidor =
            ServidorDoCanal::iniciar(CAMINHO_DO_CANAL, destino).map_err(ErroDaSuite::Canal)?;
        Ok(Self {
            plots,
            trabalho,
            filas: Filas::novo(PowerShell::do_sistema(), pastas),
            abertas: HashMap::new(),
            pedidos,
            servidor: Some(servidor),
            filas_antes,
            parar,
        })
    }

    /// Garante a fila de um modo. A fila que já existia antes da suíte não é dela: recusa, em vez de
    /// mandar trabalhos ou apagá-la depois.
    fn garantir_fila(&mut self, modo: &ModoDaFila, posicao: usize) -> Result<String, ErroDaSuite> {
        let curto = format!("Suite {} {}", modo.codigo, posicao + 1);
        if let Some(aberta) = self.abertas.get(&curto) {
            return Ok(aberta.nome.clone());
        }
        let nome =
            NomeVirtual::novo(&curto).map_err(|erro| ErroDaSuite::Filas(erro.to_string()))?;
        if self.filas_antes.contains(&nome.fila()) {
            return Err(ErroDaSuite::Filas(format!(
                "a fila \"{}\" já existia antes da suíte; remova-a antes de rodar",
                nome.fila()
            )));
        }
        let criada = self
            .filas
            .criar(&curto, modo.modo_fisico())
            .map_err(erro_de_filas)?;
        let (destino, eventos) = mpsc::channel();
        let recepcao = Recepcao::nova(
            SpoolerDoWindows,
            nome.fila(),
            PathBuf::from(&criada.porta),
            self.filas.pastas().historico(&nome),
        )
        .map_err(|erro| ErroDaSuite::Filas(erro.to_string()));
        let observador = recepcao.and_then(|recepcao| {
            Observador::iniciar(recepcao, destino)
                .map_err(|erro| ErroDaSuite::Filas(erro.to_string()))
        });
        let observador = match observador {
            Ok(observador) => observador,
            Err(erro) => {
                let _ = self.filas.remover(&curto);
                return Err(erro);
            }
        };
        let fila = nome.fila();
        self.abertas.insert(
            curto.clone(),
            FilaAberta {
                curto,
                nome: fila.clone(),
                eventos,
                observador: Some(observador),
            },
        );
        Ok(fila)
    }

    /// Roda um cenário. Falha do cenário vira resultado; só falha do ambiente interrompe a matriz.
    pub fn rodar(&mut self, cenario: &Cenario) -> Result<ResultadoDoCenario, ErroDaSuite> {
        let inicio = Instant::now();
        let mut nomes = Vec::with_capacity(cenario.modos.len());
        let mut posicoes: HashMap<&str, usize> = HashMap::new();
        for modo in &cenario.modos {
            let posicao = posicoes.entry(modo.codigo).or_insert(0);
            nomes.push(self.garantir_fila(modo, *posicao)?);
            *posicao += 1;
        }
        let pasta = self.trabalho.join("cenario");
        let _ = fs::remove_dir_all(&pasta);
        fs::create_dir_all(&pasta).map_err(|erro| ErroDaSuite::Pasta(pasta.clone(), erro))?;
        self.descartar_sobras();
        let saida = self.executar(cenario, &nomes, &pasta);
        let mut resultado = match saida {
            Ok(apurado) => apurar(cenario, &nomes, apurado),
            Err(ErroDaSuite::Cancelada) => return Err(ErroDaSuite::Cancelada),
            Err(erro) => ResultadoDoCenario {
                nome: cenario.nome(),
                estado: Estado::Falhou,
                motivo: erro.to_string(),
                macos: Vec::new(),
                motivos: Vec::new(),
                duracao: Duration::ZERO,
            },
        };
        resultado.duracao = inicio.elapsed();
        self.limpar_recebidos(&nomes);
        self.recriar_se_presa(&nomes, cenario)?;
        let _ = fs::remove_dir_all(&pasta);
        Ok(resultado)
    }

    /// Eventos e pedidos de um cenário anterior não contam neste.
    fn descartar_sobras(&self) {
        for aberta in self.abertas.values() {
            while aberta.eventos.try_recv().is_ok() {}
        }
        while let Ok(pedido) = self.pedidos.try_recv() {
            pedido.responder(Err("cenário encerrado".to_string()));
        }
    }

    /// Os PDFs recebidos já foram lidos: histórico sem limpeza encheria o disco.
    fn limpar_recebidos(&self, nomes: &[String]) {
        for nome in nomes {
            if let Ok(virtual_) = NomeVirtual::da_fila(nome) {
                let historico = self.filas.pastas().historico(&virtual_);
                if let Ok(arquivos) = fs::read_dir(&historico) {
                    for arquivo in arquivos.flatten() {
                        let _ = fs::remove_file(arquivo.path());
                    }
                }
            }
        }
    }

    /// Um trabalho preso atrapalharia o próximo cenário: a fila sai e é criada de novo.
    fn recriar_se_presa(&mut self, nomes: &[String], cenario: &Cenario) -> Result<(), ErroDaSuite> {
        for (nome, modo) in nomes.iter().zip(&cenario.modos) {
            let presa = SpoolerDoWindows
                .trabalhos(nome)
                .map(|trabalhos| !trabalhos.is_empty())
                .unwrap_or(true);
            if !presa {
                continue;
            }
            let Some(curto) = self
                .abertas
                .iter()
                .find(|(_, aberta)| aberta.nome == *nome)
                .map(|(curto, _)| curto.clone())
            else {
                continue;
            };
            if let Some(mut aberta) = self.abertas.remove(&curto) {
                if let Some(observador) = aberta.observador.take() {
                    observador.parar();
                }
                remover_com_paciencia(&self.filas, &curto).map_err(ErroDaSuite::Filas)?;
            }
            let posicao = curto
                .rsplit(' ')
                .next()
                .and_then(|texto| texto.parse::<usize>().ok())
                .unwrap_or(1)
                .saturating_sub(1);
            self.garantir_fila(modo, posicao)?;
        }
        Ok(())
    }

    fn preparar_perfil(&self, modo: &ModoDaFila, fila: &str, arquivo: &Path) -> Result<(), String> {
        match modo.perfil {
            PerfilDaFila::Modelo(codigo) => {
                let mut comando = Command::new(&self.plots);
                comando
                    .args(["perfil", fila, "--modelo", codigo, "--perfis"])
                    .arg(arquivo)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                let mut filho = comando
                    .spawn()
                    .map_err(|erro| format!("não deu para rodar o plots: {erro}"))?;
                let status = esperar_com_prazo(&mut filho, PRAZO_DO_COMANDO)
                    .ok_or("'plots perfil' passou do prazo")?;
                if status.success() {
                    Ok(())
                } else {
                    Err(format!("'plots perfil' saiu com {status}"))
                }
            }
            PerfilDaFila::Salvo { .. } => {
                let texto = modo.perfil.toml(fila).ok_or("perfil sem texto")?;
                fs::write(arquivo, texto).map_err(|erro| erro.to_string())
            }
        }
    }

    /// Começa os envios e atende cada fila até o fim, ou até o prazo.
    fn executar(
        &mut self,
        cenario: &Cenario,
        nomes: &[String],
        pasta: &Path,
    ) -> Result<Apurado, ErroDaSuite> {
        let documento = pasta.join("documento.pdf");
        let bytes = pdf_de_teste(&cenario.paginas_do_documento()).map_err(ErroDaSuite::Filas)?;
        fs::write(&documento, bytes).map_err(|erro| ErroDaSuite::Pasta(documento.clone(), erro))?;
        let total_filas = nomes.len();
        let mut bancadas: Vec<EnvioEmCurso> = Vec::with_capacity(total_filas);
        for (indice, (nome, modo)) in nomes.iter().zip(&cenario.modos).enumerate() {
            let perfis = pasta.join(format!("perfis-{indice}.toml"));
            if let Err(erro) = self.preparar_perfil(modo, nome, &perfis) {
                return Ok(Apurado::com_falha(bancadas, erro));
            }
            let mut comando = Command::new(&self.plots);
            comando
                .arg("imprimir")
                .arg(&documento)
                .args(["--impressora", nome, "--lote"])
                .arg(cenario.lote_pedido().to_string())
                .arg("--perfis")
                .arg(&perfis)
                .arg("--teste");
            if cenario.reparto == Reparto::Divisao {
                comando
                    .arg("--parte")
                    .arg(format!("{}/{total_filas}", indice + 1));
            }
            let log = pasta.join(format!("plots-{indice}.log"));
            let (saida, erros) = abrir_logs(&log)?;
            comando.stdin(Stdio::null()).stdout(saida).stderr(erros);
            let filho = match comando.spawn() {
                Ok(filho) => filho,
                Err(erro) => {
                    return Ok(Apurado::com_falha(
                        bancadas,
                        format!("não deu para rodar o plots: {erro}"),
                    ))
                }
            };
            bancadas.push(EnvioEmCurso {
                fila: nome.clone(),
                bancada: Bancada::nova(modo.modo, cenario.erro),
                filho: Some(filho),
                status: None,
                log,
                trabalhos: VecDeque::new(),
                saiu_em: None,
                recebeu_inicio: false,
            });
        }
        let prazo = PRAZO_BASE + PRAZO_POR_PAGINA * cenario.paginas;
        let inicio = Instant::now();
        let mut pendentes: Vec<Pedido> = Vec::new();
        let mut falhas = Vec::new();
        let curto_de: HashMap<String, String> = self
            .abertas
            .values()
            .map(|aberta| (aberta.nome.clone(), aberta.curto.clone()))
            .collect();
        loop {
            if self.parar.load(Ordering::Relaxed) {
                encerrar_filhos(&mut bancadas);
                return Err(ErroDaSuite::Cancelada);
            }
            for envio in &mut bancadas {
                let Some(aberta) = curto_de
                    .get(&envio.fila)
                    .and_then(|curto| self.abertas.get(curto))
                else {
                    continue;
                };
                colher_eventos(aberta, envio, &mut falhas);
            }
            loop {
                match self.pedidos.try_recv() {
                    Ok(pedido) => pendentes.push(pedido),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        falhas.push("o canal do simulador caiu".to_string());
                        break;
                    }
                }
            }
            pendentes = atender(pendentes, &mut bancadas, cenario, &mut falhas);
            for envio in &mut bancadas {
                receber_verso(envio, cenario, &mut falhas);
                envio.olhar_processo();
            }
            let todos_sairam = bancadas.iter().all(|envio| envio.filho.is_none());
            if todos_sairam {
                let bem = bancadas.iter().all(EnvioEmCurso::fechou);
                let folga_acabou = bancadas.iter().all(|envio| {
                    envio
                        .saiu_em
                        .is_some_and(|quando| quando.elapsed() > FOLGA_DEPOIS_DE_SAIR)
                });
                if bem || folga_acabou {
                    break;
                }
            }
            if inicio.elapsed() > prazo {
                falhas.push(format!(
                    "trabalho preso: o cenário passou do prazo de {} s",
                    prazo.as_secs()
                ));
                break;
            }
            thread::sleep(INTERVALO);
        }
        for pedido in pendentes {
            pedido.responder(Err("cenário encerrado".to_string()));
        }
        encerrar_filhos(&mut bancadas);
        Ok(Apurado { bancadas, falhas })
    }
}

impl Drop for Ambiente {
    fn drop(&mut self) {
        if let Some(servidor) = self.servidor.take() {
            servidor.parar();
        }
        for (curto, mut aberta) in std::mem::take(&mut self.abertas) {
            if let Some(observador) = aberta.observador.take() {
                observador.parar();
            }
            if let Err(erro) = remover_com_paciencia(&self.filas, &curto) {
                eprintln!("não deu para remover a fila {curto}: {erro}");
            }
        }
        let _ = fs::remove_dir_all(&self.trabalho);
    }
}

/// O spooler pode segurar a fila por um instante depois do último trabalho.
fn remover_com_paciencia(filas: &Filas<Sistema>, curto: &str) -> Result<(), String> {
    let mut ultimo = String::new();
    for tentativa in 0..TENTATIVAS_DE_REMOVER {
        if tentativa > 0 {
            thread::sleep(PAUSA_ENTRE_REMOCOES);
        }
        match filas.remover(curto) {
            Ok(_) => return Ok(()),
            Err(erro) => ultimo = erro.to_string(),
        }
    }
    Err(ultimo)
}

fn abrir_logs(log: &Path) -> Result<(File, File), ErroDaSuite> {
    let falha = |erro| ErroDaSuite::Pasta(log.to_path_buf(), erro);
    let arquivo = File::create(log).map_err(falha)?;
    let copia = arquivo.try_clone().map_err(falha)?;
    Ok((arquivo, copia))
}

fn esperar_com_prazo(filho: &mut Child, prazo: Duration) -> Option<ExitStatus> {
    let inicio = Instant::now();
    loop {
        match filho.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if inicio.elapsed() > prazo => {
                let _ = filho.kill();
                let _ = filho.wait();
                return None;
            }
            Ok(None) => thread::sleep(INTERVALO),
            Err(_) => return None,
        }
    }
}

/// O envio de uma fila em andamento.
struct EnvioEmCurso {
    fila: String,
    bancada: Bancada,
    filho: Option<Child>,
    status: Option<ExitStatus>,
    log: PathBuf,
    trabalhos: VecDeque<TrabalhoRecebido>,
    saiu_em: Option<Instant>,
    recebeu_inicio: bool,
}

impl EnvioEmCurso {
    fn olhar_processo(&mut self) {
        if let Some(filho) = &mut self.filho {
            if let Ok(Some(status)) = filho.try_wait() {
                self.status = Some(status);
                self.filho = None;
                self.saiu_em = Some(Instant::now());
            }
        }
    }

    /// Terminou e já mandou tudo, ou não tinha nada a mandar nesta parte.
    fn fechou(&self) -> bool {
        self.status.is_some_and(|status| status.success())
            && (self.bancada.etapa() == Etapa::Encerrada || !self.recebeu_inicio)
    }

    fn ultimas_linhas(&self) -> String {
        let texto = fs::read(&self.log).unwrap_or_default();
        motivo_no_log(&String::from_utf8_lossy(&texto))
    }

    /// A linha de recusa do Ofício, se o processo saiu com erro e o log a traz.
    fn linha_da_recusa(&self) -> Option<String> {
        if !self.status.is_some_and(|status| !status.success()) {
            return None;
        }
        let bytes = fs::read(&self.log).ok()?;
        let texto = String::from_utf8_lossy(&bytes).into_owned();
        linha_de_recusa(&texto).map(str::to_string)
    }
}

/// O que o PlotS disse ao falhar: a linha de erro, que vem antes do texto de uso. Sem ela, as
/// últimas linhas.
fn motivo_no_log(texto: &str) -> String {
    if let Some(erro) = texto.lines().find(|linha| linha.starts_with("Erro: ")) {
        return erro.to_string();
    }
    let linhas: Vec<&str> = texto.lines().filter(|l| !l.trim().is_empty()).collect();
    let de = linhas.len().saturating_sub(3);
    linhas.get(de..).unwrap_or(&[]).join(" | ")
}

fn encerrar_filhos(bancadas: &mut [EnvioEmCurso]) {
    for envio in bancadas {
        if let Some(mut filho) = envio.filho.take() {
            let _ = filho.kill();
            envio.status = filho.wait().ok();
        }
    }
}

fn colher_eventos(aberta: &FilaAberta, envio: &mut EnvioEmCurso, falhas: &mut Vec<String>) {
    while let Ok(evento) = aberta.eventos.try_recv() {
        match evento {
            Evento::Recebido(recebido) => envio.trabalhos.push_back(recebido),
            Evento::Falhou { documento, .. } => falhas.push(format!(
                "o spooler marcou falha no trabalho \"{documento}\" de {}",
                envio.fila
            )),
            Evento::SemArquivo { documento, .. } => falhas.push(format!(
                "o trabalho \"{documento}\" de {} saiu da fila sem arquivo",
                envio.fila
            )),
            Evento::ArquivoRecusado { bytes } => falhas.push(format!(
                "um arquivo de {} tem {bytes} bytes, acima do limite",
                envio.fila
            )),
            Evento::Aviso(aviso) => falhas.push(format!("{}: {aviso}", envio.fila)),
        }
    }
}

/// Lê um trabalho recebido e devolve as páginas na ordem em que saíram. Confere o papel: o spooler
/// tem de entregar a medida do documento.
fn ler_trabalho(
    trabalho: &TrabalhoRecebido,
    cenario: &Cenario,
    falhas: &mut Vec<String>,
) -> Result<Vec<Envio>, String> {
    let paginas =
        ler_marcas(&trabalho.arquivo).map_err(|erro| format!("{}: {erro}", trabalho.documento))?;
    let mut envios = Vec::with_capacity(paginas.len());
    for pagina in &paginas {
        let envio = envio_da_leitura(&pagina.leitura).map_err(|erro| {
            format!(
                "{}, página {} do arquivo: {erro}",
                trabalho.documento, pagina.numero
            )
        })?;
        conferir_papel(trabalho, pagina, envio, cenario, falhas);
        envios.push(envio);
    }
    Ok(envios)
}

fn conferir_papel(
    trabalho: &TrabalhoRecebido,
    pagina: &PaginaMarcada,
    envio: Envio,
    cenario: &Cenario,
    falhas: &mut Vec<String>,
) {
    let Some(numero) = envio.pagina else {
        return;
    };
    let formato = cenario.formato_da_pagina(numero);
    let (largura, altura) = formato.milimetros();
    let certo = (pagina.largura_mm - largura).abs() <= TOLERANCIA_MM
        && (pagina.altura_mm - altura).abs() <= TOLERANCIA_MM;
    if !certo {
        falhas.push(format!(
            "{}: a página {numero} devia sair em {} ({largura:.0} × {altura:.0} mm) e saiu em {:.0} × {:.0} mm",
            trabalho.documento,
            formato.nome(),
            pagina.largura_mm,
            pagina.altura_mm,
        ));
    }
}

/// Atende o que dá; o que precisa de um trabalho que ainda não chegou fica para o próximo passo.
fn atender(
    pendentes: Vec<Pedido>,
    bancadas: &mut [EnvioEmCurso],
    cenario: &Cenario,
    falhas: &mut Vec<String>,
) -> Vec<Pedido> {
    let mut restantes = Vec::new();
    for pedido in pendentes {
        let Some(envio) = bancadas
            .iter_mut()
            .find(|envio| envio.fila == pedido.mensagem.fila())
        else {
            pedido.responder(Err("esta fila não está em um cenário agora".to_string()));
            continue;
        };
        match &pedido.mensagem {
            Mensagem::Inicio { .. } => {
                envio.recebeu_inicio = true;
                pedido.responder(Ok(()));
            }
            Mensagem::Gesto { lote, gesto, .. } => {
                let (lote, gesto) = (*lote, *gesto);
                let Some(trabalho) = envio.trabalhos.pop_front() else {
                    restantes.push(pedido);
                    continue;
                };
                let resposta = ler_trabalho(&trabalho, cenario, falhas)
                    .and_then(|frente| envio.bancada.frente_e_gesto(lote, &frente, gesto));
                if let Err(erro) = &resposta {
                    falhas.push(format!("{}: {erro}", envio.fila));
                }
                pedido.responder(resposta);
            }
            Mensagem::Fim { lotes, leitura, .. } => {
                let (lotes, leitura): (u32, LeituraPedida) = (*lotes, *leitura);
                if matches!(envio.bancada.etapa(), Etapa::EsperandoVerso { .. }) {
                    restantes.push(pedido);
                    continue;
                }
                let resposta = envio.bancada.encerrar(lotes, leitura);
                if let Err(erro) = &resposta {
                    falhas.push(format!("{}: {erro}", envio.fila));
                }
                pedido.responder(resposta);
            }
        }
    }
    restantes
}

fn receber_verso(envio: &mut EnvioEmCurso, cenario: &Cenario, falhas: &mut Vec<String>) {
    if !matches!(envio.bancada.etapa(), Etapa::EsperandoVerso { .. }) {
        return;
    }
    let Some(trabalho) = envio.trabalhos.pop_front() else {
        return;
    };
    let resultado =
        ler_trabalho(&trabalho, cenario, falhas).and_then(|verso| envio.bancada.verso(&verso));
    if let Err(erro) = resultado {
        falhas.push(format!("{}: {erro}", envio.fila));
    }
}

/// O que sobrou do cenário para julgar.
struct Apurado {
    bancadas: Vec<EnvioEmCurso>,
    falhas: Vec<String>,
}

impl Apurado {
    fn com_falha(mut bancadas: Vec<EnvioEmCurso>, falha: String) -> Self {
        encerrar_filhos(&mut bancadas);
        Self {
            bancadas,
            falhas: vec![falha],
        }
    }
}

fn texto_dos_motivos(motivos: &[Motivo]) -> String {
    motivos
        .iter()
        .take(6)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// Julga o cenário: o PlotS saiu bem, cada folha foi lida e o maço confere. Com erro de propósito,
/// só passa quando o erro certo foi acusado.
fn apurar(cenario: &Cenario, nomes: &[String], apurado: Apurado) -> ResultadoDoCenario {
    let Apurado {
        bancadas,
        mut falhas,
    } = apurado;
    let recusas: Vec<bool> = bancadas
        .iter()
        .map(|envio| {
            recusa_limpa(
                envio.linha_da_recusa().is_some(),
                !envio.bancada.faltas().is_empty(),
                !envio.bancada.pacotes().is_empty(),
            )
        })
        .collect();
    if cenario.erro.is_none()
        && e_sem_cobertura(papel_tem_oficio(cenario), falhas.is_empty(), &recusas)
    {
        let linha = bancadas
            .iter()
            .find_map(EnvioEmCurso::linha_da_recusa)
            .unwrap_or_default();
        return ResultadoDoCenario {
            nome: cenario.nome(),
            estado: Estado::SemCobertura,
            motivo: format!("{MOTIVO_SEM_COBERTURA}. {linha}"),
            macos: Vec::new(),
            motivos: Vec::new(),
            duracao: Duration::ZERO,
        };
    }
    for envio in &bancadas {
        match envio.status {
            Some(status) if status.success() => {}
            Some(status) => falhas.push(format!(
                "{}: o plots saiu com {status}: {}",
                envio.fila,
                envio.ultimas_linhas()
            )),
            None => falhas.push(format!("{}: o plots não terminou", envio.fila)),
        }
        falhas.extend(
            envio
                .bancada
                .faltas()
                .iter()
                .filter(|_| cenario.erro.is_none())
                .cloned(),
        );
    }
    let mut macos: Vec<(String, MacoLido)> = Vec::new();
    let mut motivos = Vec::new();
    match cenario.reparto {
        Reparto::Uma | Reparto::Espelho => {
            for (envio, nome) in bancadas.iter().zip(nomes) {
                let maco = juntar(envio.bancada.pacotes().into_values());
                motivos.extend(conferir_maco(&maco, cenario.paginas));
                macos.push((nome.clone(), maco));
            }
            if let [(_, primeiro), (_, segundo)] = macos.as_slice() {
                if primeiro != segundo {
                    falhas
                        .push("o mesmo documento deu maços diferentes nas duas filas".to_string());
                }
            }
        }
        Reparto::Divisao => {
            let mut todos: BTreeMap<u32, MacoLido> = BTreeMap::new();
            for envio in &bancadas {
                todos.extend(envio.bancada.pacotes());
            }
            let maco = juntar(todos.into_values());
            motivos.extend(conferir_maco(&maco, cenario.paginas));
            macos.push(("Pacotes juntos".to_string(), maco));
        }
    }
    let faltas: Vec<String> = bancadas
        .iter()
        .flat_map(|envio| envio.bancada.faltas().iter().cloned())
        .collect();
    let (passou, motivo) = match cenario.erro {
        None if falhas.is_empty() && motivos.is_empty() => (true, "o maço confere".to_string()),
        None => {
            // Se o envio não terminou, o maço incompleto é consequência, não mais um motivo.
            if falhas.is_empty() {
                (false, texto_dos_motivos(&motivos))
            } else {
                (false, falhas.join("; "))
            }
        }
        Some(erro) => {
            if !falhas.is_empty() {
                (false, format!("{}: {}", erro.nome(), falhas.join("; ")))
            } else if erro_foi_acusado(erro, &motivos, &faltas) {
                (
                    true,
                    format!("acusou como devia: {}", texto_dos_motivos(&motivos)),
                )
            } else {
                (
                    false,
                    format!(
                        "o erro de propósito não foi acusado: {}",
                        texto_dos_motivos(&motivos)
                    ),
                )
            }
        }
    };
    ResultadoDoCenario {
        nome: cenario.nome(),
        estado: if passou {
            Estado::Passou
        } else {
            Estado::Falhou
        },
        motivo,
        macos,
        motivos,
        duracao: Duration::ZERO,
    }
}

/// Motivo de um cenário sem cobertura. Vem antes da linha que o PlotS escreveu.
const MOTIVO_SEM_COBERTURA: &str =
    "o driver da fila não tem este papel; o PlotS recusou sem enviar nada";

/// O papel do cenário tem alguma página de Ofício.
fn papel_tem_oficio(cenario: &Cenario) -> bool {
    (1..=cenario.paginas).any(|numero| cenario.formato_da_pagina(numero) == Formato::Oficio)
}

/// O trecho que o PlotS escreve quando a fila não aplica o papel de Ofício (216 × 330 mm). Vem
/// depois do texto do lote, dentro da linha de erro.
const TRECHO_DA_RECUSA_DO_OFICIO: &str = "a fila não aplicou o papel de 216 × 330 mm e ficou em ";

/// A linha de erro do PlotS quando a fila não aplica o papel de Ofício e nada é enviado. A medida
/// que ficou tem de ser outra, senão não é recusa.
fn e_recusa_do_oficio(linha: &str) -> bool {
    let Some(texto) = linha.strip_prefix("Erro: ") else {
        return false;
    };
    texto
        .find(TRECHO_DA_RECUSA_DO_OFICIO)
        .and_then(|inicio| texto.get(inicio + TRECHO_DA_RECUSA_DO_OFICIO.len()..))
        .and_then(|resto| resto.strip_suffix("; nada foi enviado"))
        .is_some_and(|obtido| {
            obtido.contains(" × ") && obtido.ends_with(" mm") && obtido != "216 × 330 mm"
        })
}

/// A linha de recusa do Ofício dentro do log do processo, se houver.
fn linha_de_recusa(texto: &str) -> Option<&str> {
    texto.lines().find(|linha| e_recusa_do_oficio(linha))
}

/// Uma fila conta como recusada sem envio só se saiu com a recusa, sem falta e sem nada recebido.
/// Recusa depois de um envio parcial é falha: o que chegou ainda precisa ser conferido.
fn recusa_limpa(recusou: bool, tem_faltas: bool, recebeu: bool) -> bool {
    recusou && !tem_faltas && !recebeu
}

/// Sem cobertura, e não falha: o papel tem Ofício, nada mais deu errado, e cada fila saiu com a
/// recusa do Ofício. Qualquer outro caso continua falha.
fn e_sem_cobertura(papel_com_oficio: bool, sem_outras_falhas: bool, recusas: &[bool]) -> bool {
    papel_com_oficio
        && sem_outras_falhas
        && !recusas.is_empty()
        && recusas.iter().all(|recusa| *recusa)
}

/// O que sobrou no sistema com o prefixo das filas da suíte, em texto para o relatório.
pub fn filas_da_suite_no_sistema() -> Result<Vec<String>, String> {
    let filas = PowerShell::do_sistema()
        .filas()
        .map_err(|erro| erro.to_string())?;
    Ok(filas
        .into_iter()
        .map(|fila| fila.nome)
        .filter(|nome| nome.starts_with("PlotSim — Suite "))
        .collect())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::matriz::{Cenario, Papel, Reparto, JATO};

    const RECUSA_DE_OFICIO: &str = "Erro: a fila não aplicou o papel de 216 × 330 mm e ficou em 210 × 297 mm; nada foi enviado";

    fn cenario(paginas: u32, papel: Papel) -> Cenario {
        Cenario {
            modos: vec![JATO],
            reparto: Reparto::Uma,
            paginas,
            lote: Some(2),
            papel,
            erro: None,
        }
    }

    #[test]
    fn o_motivo_e_a_linha_de_erro_e_nao_o_texto_de_uso() {
        let log = "Lote 1\nErro: a fila recusou o papel\n\nUso:\n  plots imprimir\n";
        assert_eq!(motivo_no_log(log), "Erro: a fila recusou o papel");
        assert_eq!(motivo_no_log("a\nb\nc\nd\n"), "b | c | d");
        assert_eq!(motivo_no_log(""), "");
    }

    #[test]
    fn so_conta_como_papel_com_oficio_o_que_tem_pagina_de_oficio() {
        assert!(papel_tem_oficio(&cenario(6, Papel::Um(Formato::Oficio))));
        assert!(!papel_tem_oficio(&cenario(6, Papel::Um(Formato::A4))));
        assert!(!papel_tem_oficio(&cenario(6, Papel::Misto)));
        assert!(!papel_tem_oficio(&cenario(7, Papel::Misto)));
    }

    #[test]
    fn reconhece_a_recusa_do_oficio_com_a_medida_pedida_e_nao_outra() {
        assert!(e_recusa_do_oficio(RECUSA_DE_OFICIO));
        assert!(!e_recusa_do_oficio(
            "Erro: a fila não aplicou o papel de 210 × 297 mm e ficou em 216 × 330 mm; nada foi enviado"
        ));
        assert!(!e_recusa_do_oficio(
            "Erro: a fila não aplicou o papel de 216 × 330 mm e ficou em 210 × 297 mm"
        ));
        assert!(!e_recusa_do_oficio(
            &RECUSA_DE_OFICIO.replacen("Erro: ", "", 1)
        ));
        assert!(!e_recusa_do_oficio(
            "Erro: o driver não aceitou imprimir em uma face só; nada foi enviado"
        ));
    }

    #[test]
    fn a_recusa_chega_dentro_da_linha_de_erro_do_envio() {
        // Linha real de uma execução: o PlotS põe a recusa depois do texto do lote.
        let real = "Erro: não foi possível enviar a frente do lote 1: a fila não aplicou o papel de 216 × 330 mm e ficou em 210 × 297 mm; nada foi enviado";
        assert!(e_recusa_do_oficio(real));
        assert!(!e_recusa_do_oficio(
            "Erro: não foi possível enviar a frente do lote 1: a fila não aplicou o papel de 210 × 297 mm e ficou em 216 × 330 mm; nada foi enviado"
        ));
    }

    #[test]
    fn a_linha_de_recusa_sai_do_log_inteiro() {
        let log = format!("Lote 1\n{RECUSA_DE_OFICIO}\n\nUso:\n");
        assert_eq!(linha_de_recusa(&log), Some(RECUSA_DE_OFICIO));
        assert_eq!(linha_de_recusa("Lote 1\nUso:\n"), None);
    }

    #[test]
    fn a_recusa_do_oficio_sem_outra_falha_e_sem_cobertura() {
        assert!(e_sem_cobertura(true, true, &[true, true]));
        assert!(e_sem_cobertura(true, true, &[true]));
    }

    #[test]
    fn a_mesma_recusa_fora_do_oficio_continua_falha() {
        assert!(!e_sem_cobertura(false, true, &[true]));
    }

    #[test]
    fn outra_falha_junto_da_recusa_continua_falha() {
        assert!(!e_sem_cobertura(true, false, &[true]));
    }

    #[test]
    fn uma_fila_que_nao_recusou_continua_falha() {
        assert!(!e_sem_cobertura(true, true, &[true, false]));
    }

    #[test]
    fn recusa_depois_de_envio_parcial_nao_e_sem_envio() {
        assert!(!recusa_limpa(true, false, true));
    }

    #[test]
    fn recusa_sem_nada_recebido_e_limpa() {
        assert!(recusa_limpa(true, false, false));
    }

    #[test]
    fn falta_junto_da_recusa_nao_e_limpa() {
        assert!(!recusa_limpa(true, true, false));
    }

    #[test]
    fn sem_fila_recusada_nao_ha_sem_cobertura() {
        assert!(!e_sem_cobertura(true, true, &[]));
    }
}
