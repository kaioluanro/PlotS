//! `imprimir` sem `--simular`: conduz o envio real, lote a lote, com uma confirmação por envio.
//!
//! A saída nunca diz que algo saiu no papel: a confirmação do spooler só garante a entrega.

use crate::calibrar::{caminho_perfis, carregar_perfis, MSG_RESPOSTA_INVALIDA};
use crate::geracao::{self, Documento, LoteDoPlano};
use crate::{listar, ImpressoraVirtual};
use plots_core::{FormatoPapel, OrdemPassadas, Papel, TOLERANCIA_PAPEL_MM};
use plots_print::job::{Backend, ErroJob, EstadoJob, Job};
use plots_windows::Fila;
use std::fmt::Display;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::time::Duration;

/// Recusa de `imprimir` sem `--simular` quando o nome é o de uma impressora simulada.
pub(crate) const MSG_SEM_SIMULAR: &str =
    "as impressoras simuladas só existem com --simular; para enviar de verdade, informe uma fila com perfil salvo";

const FRASE_ESPERAR_FOLHA: &str = "Espere a última folha sair antes de recolocar o maço.";
const PERGUNTA_CANCELAR: &str = "Cancelar o trabalho na fila? (s/n): ";
const MSG_CANCELADO: &str = "O trabalho foi cancelado ou já tinha saído da fila; folhas já entregues ao dispositivo ainda podem sair. Nada mais será enviado.";
const MSG_DEIXADO_NA_FILA: &str =
    "O PlotS não apagou o trabalho; veja o estado com 'plots filas'. Nada mais será enviado.";
/// Meia hora de consultas, uma a cada segundo: passa disso, o comando devolve o terminal. A
/// espera por alguém resolver um aviso da impressora não conta.
#[cfg_attr(not(windows), allow(dead_code))]
const ESPERA_REAL: Espera = Espera {
    intervalo: Duration::from_secs(1),
    consultas: 1_800,
};
/// Consultas com erro, em seguida, antes de desistir de acompanhar o trabalho.
const ERROS_DE_CONSULTA_SEGUIDOS: u32 = 30;

/// O que `imprimir` recebeu para um envio real.
pub(crate) struct Pedido {
    pub arquivo: String,
    pub nome: String,
    pub tamanho: usize,
    pub ordem: OrdemPassadas,
    pub perfis: Option<PathBuf>,
    pub arquivo_saida: Option<PathBuf>,
}

/// Motivo que o backend sabe dar para a falha de um trabalho ou para o que a impressora espera
/// que alguém resolva. Vazio quando o trabalho anda sem problema.
pub(crate) trait DescreveFalha {
    fn motivo(&self) -> Option<String>;
    /// O trabalho já não está na fila, e por isso não há o que apagar nem cancelar.
    fn trabalho_saiu_da_fila(&self) -> bool;
}

#[cfg(windows)]
impl DescreveFalha for plots_windows::BackendWindows {
    fn motivo(&self) -> Option<String> {
        self.motivo_atual().map(str::to_string)
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        self.saiu_da_fila()
    }
}

/// Com que frequência perguntar à fila e até quando esperar.
#[cfg_attr(not(windows), allow(dead_code))]
struct Espera {
    intervalo: Duration,
    consultas: u32,
}

/// O que o fluxo precisa saber da fila, sem depender do Windows.
#[cfg_attr(not(windows), allow(dead_code))]
struct Destino<'a> {
    fila: &'a str,
    gesto_texto: Option<&'a str>,
}

/// Executa o envio real: nada é enviado sem a resposta do usuário.
pub(crate) fn executar(
    pedido: Pedido,
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> Result<(), String> {
    if ImpressoraVirtual::de_nome(&pedido.nome).is_some() {
        return Err(MSG_SEM_SIMULAR.into());
    }
    if pedido.tamanho == 0 {
        return Err("--lote deve ser maior que zero".into());
    }
    if let Some(pasta) = &pedido.arquivo_saida {
        if !pasta.is_dir() {
            return Err("a pasta de --arquivo-saida deve existir".into());
        }
    }
    let lidos = carregar_perfis(&caminho_perfis(pedido.perfis.clone())?)?;
    let perfil = lidos.obter(&pedido.nome).ok_or_else(|| {
        format!(
            "a impressora '{0}' não tem perfil salvo; rode 'plots calibrar {0} --saida DIRETORIO' antes",
            pedido.nome
        )
    })?;
    // A barreira de PDF protegido vem antes de olhar a fila ou a biblioteca de desenho.
    let documento = Documento::ler(std::path::Path::new(&pedido.arquivo))?;
    let plano = geracao::planejar(&documento, pedido.tamanho, pedido.ordem, perfil.perfil)?;
    let fila = conferir_fila(&pedido, saida)?;
    writeln!(
        saida,
        "ENVIO À IMPRESSORA — {}\n{} páginas — {} lote(s)",
        pedido.nome,
        documento.paginas(),
        plano.lotes.len()
    )
    .map_err(|e| e.to_string())?;
    geracao::escrever_avisos(&documento, &plano, pedido.ordem, saida)?;
    if let Some(pasta) = &pedido.arquivo_saida {
        writeln!(
            saida,
            "As passadas vão para arquivos em {}, pela própria fila.",
            pasta.display()
        )
        .map_err(|e| e.to_string())?;
    }
    let lotes = geracao::lotes_do_plano(&plano);
    let destino = Destino {
        fila: &pedido.nome,
        gesto_texto: perfil.gesto_texto.as_deref(),
    };
    enviar_com_o_sistema(&pedido, &fila, documento, lotes, &destino, entrada, saida)
}

/// Escolhe a fila do pedido na lista do sistema. O nome precisa ser exatamente igual. Fax nunca
/// serve, e uma fila que pergunta o nome do arquivo só serve com arquivo de saída: sem ele, o
/// Windows abriria um diálogo e o envio ficaria parado.
fn decidir_fila<'a>(
    filas: &'a [Fila],
    nome: &str,
    tem_arquivo_saida: bool,
) -> Result<&'a Fila, String> {
    let fila = filas.iter().find(|fila| fila.nome == nome).ok_or_else(|| {
        format!("a fila '{nome}' não existe neste sistema; veja os nomes com 'plots filas'")
    })?;
    if plots_windows::porta_de_fax(&fila.porta) {
        return Err(format!(
            "a fila '{nome}' é um fax compartilhado; o envio não usa fax"
        ));
    }
    if plots_windows::porta_pede_arquivo(&fila.porta) && !tem_arquivo_saida {
        return Err(format!(
            "a fila '{nome}' pede o nome do arquivo em um diálogo; informe --arquivo-saida PASTA"
        ));
    }
    Ok(fila)
}

/// Lê as filas do sistema, decide e avisa se a escolhida não está pronta.
fn conferir_fila(pedido: &Pedido, saida: &mut impl Write) -> Result<Fila, String> {
    let filas = plots_windows::listar_filas().map_err(|e| e.to_string())?;
    let fila = decidir_fila(&filas, &pedido.nome, pedido.arquivo_saida.is_some())?;
    if !fila.estado.pronta() {
        writeln!(
            saida,
            "Aviso: a fila '{}' não está pronta agora; veja o estado com 'plots filas'.",
            pedido.nome
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(fila.clone())
}

#[cfg(windows)]
fn enviar_com_o_sistema(
    pedido: &Pedido,
    fila: &Fila,
    documento: Documento,
    lotes: Vec<LoteDoPlano>,
    destino: &Destino<'_>,
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> Result<(), String> {
    let (origem, dimensoes) = documento.em_partes();
    let backend = plots_windows::BackendWindows::novo(
        fila,
        origem,
        dimensoes,
        pedido.arquivo_saida.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    conduzir(
        backend,
        lotes,
        destino,
        &ESPERA_REAL,
        std::thread::sleep,
        entrada,
        saida,
    )
}

#[cfg(not(windows))]
fn enviar_com_o_sistema(
    _pedido: &Pedido,
    _fila: &Fila,
    _documento: Documento,
    _lotes: Vec<LoteDoPlano>,
    _destino: &Destino<'_>,
    _entrada: &mut impl BufRead,
    _saida: &mut impl Write,
) -> Result<(), String> {
    Err(crate::MSG_ENVIO_INDISPONIVEL.into())
}

/// Nome do papel como o usuário o procura na gaveta.
fn nome_do_papel(papel: &Papel) -> String {
    match papel.formato {
        FormatoPapel::A4 => "A4".to_string(),
        FormatoPapel::A5 => "A5".to_string(),
        FormatoPapel::Carta => "Carta".to_string(),
        FormatoPapel::Oficio => "Ofício".to_string(),
        FormatoPapel::Legal => "Legal".to_string(),
        FormatoPapel::Personalizado => {
            format!("{:.0} × {:.0} mm", papel.largura_mm, papel.altura_mm)
        }
    }
}

/// Dois lotes usam a mesma folha, mesmo que um seja retrato e o outro, paisagem.
fn mesma_folha(a: &Papel, b: &Papel) -> bool {
    a.formato == b.formato
        && (a.formato != FormatoPapel::Personalizado
            || ((a.largura_mm - b.largura_mm).abs() <= TOLERANCIA_PAPEL_MM
                && (a.altura_mm - b.altura_mm).abs() <= TOLERANCIA_PAPEL_MM))
}

/// Fluxo de cada lote: pergunta, frente, fim do trabalho na fila, instrução, pergunta, verso.
#[cfg_attr(not(windows), allow(dead_code))]
fn conduzir<B>(
    backend: B,
    lotes: Vec<LoteDoPlano>,
    destino: &Destino<'_>,
    espera: &Espera,
    mut dormir: impl FnMut(Duration),
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> Result<(), String>
where
    B: Backend + DescreveFalha,
    B::Erro: Display,
{
    let total = lotes.len();
    let para_o_job = lotes.iter().map(|item| item.lote.clone()).collect();
    let mut job = Job::novo(backend, para_o_job).map_err(|erro| texto_do_job(&erro))?;
    let mut papel_anterior: Option<Papel> = None;
    for (indice, LoteDoPlano { papel, lote }) in lotes.iter().enumerate() {
        let numero = indice + 1;
        let fila = destino.fila;
        let nome_papel = nome_do_papel(papel);
        writeln!(
            saida,
            "\nLote {numero} de {total}: {} folha(s), papel {nome_papel}",
            lote.passada1.len()
        )
        .map_err(|e| e.to_string())?;
        if papel_anterior.is_some_and(|anterior| !mesma_folha(&anterior, papel)) {
            writeln!(
                saida,
                "  Troque o papel para {nome_papel} e retire as folhas do lote anterior."
            )
            .map_err(|e| e.to_string())?;
        }
        papel_anterior = Some(*papel);
        writeln!(saida, "  Frente: {}", listar(&lote.passada1)).map_err(|e| e.to_string())?;
        let pergunta = format!("Enviar a frente do lote {numero} para {fila}? (s/n): ");
        if !confirmar(entrada, saida, &pergunta)? {
            return Err(format!(
                "envio cancelado no lote {numero} de {total}: a frente deste lote não foi enviada"
            ));
        }
        job.enviar_frente()
            .map_err(|erro| falha_ao_enviar("a frente", numero, &erro))?;
        escrever_aguardando(saida, "Frente", numero)?;
        let etapa = Etapa {
            passada: "a frente",
            numero,
        };
        aguardar(&mut job, espera, &mut dormir, etapa, entrada, saida)?;
        writeln!(
            saida,
            "  O trabalho da frente do lote {numero} terminou ou saiu da fila do spooler. Isso não garante que o papel já saiu."
        )
        .map_err(|e| e.to_string())?;
        writeln!(saida, "  {FRASE_ESPERAR_FOLHA}").map_err(|e| e.to_string())?;
        writeln!(
            saida,
            "  Pegue o maço como saiu, não embaralhe, e recoloque do mesmo jeito que na calibração."
        )
        .map_err(|e| e.to_string())?;
        if let Some(texto) = destino.gesto_texto {
            writeln!(saida, "  {texto}").map_err(|e| e.to_string())?;
        }
        writeln!(saida, "  Verso: {}", listar(&lote.passada2)).map_err(|e| e.to_string())?;
        let pergunta = format!("Enviar o verso do lote {numero} para {fila}? (s/n): ");
        if !confirmar(entrada, saida, &pergunta)? {
            return Err(format!(
                "envio cancelado no lote {numero} de {total}: a frente foi enviada, o verso não"
            ));
        }
        job.enviar_verso()
            .map_err(|erro| falha_ao_enviar("o verso", numero, &erro))?;
        escrever_aguardando(saida, "Verso", numero)?;
        let etapa = Etapa {
            passada: "o verso",
            numero,
        };
        aguardar(&mut job, espera, &mut dormir, etapa, entrada, saida)?;
        writeln!(
            saida,
            "  O trabalho do verso do lote {numero} terminou ou saiu da fila do spooler."
        )
        .map_err(|e| e.to_string())?;
    }
    writeln!(
        saida,
        "\nOs trabalhos dos {total} lote(s) terminaram ou saíram da fila do spooler. Confira as folhas depois que a última sair."
    )
    .map_err(|e| e.to_string())
}

fn escrever_aguardando(saida: &mut impl Write, passada: &str, numero: usize) -> Result<(), String> {
    writeln!(
        saida,
        "  {passada} do lote {numero} entregue ao spooler. Aguardando a confirmação da fila..."
    )
    .map_err(|e| e.to_string())?;
    saida.flush().map_err(|e| e.to_string())
}

/// Qual passada está sendo acompanhada, para as mensagens.
#[derive(Clone, Copy)]
struct Etapa {
    passada: &'static str,
    numero: usize,
}

/// Consulta a fila até a passada terminar. O próximo envio só é oferecido depois disso.
///
/// Um trabalho só sai da fila por decisão do usuário: erro ao consultar não diz nada sobre ele,
/// então nunca leva a cancelar. O limite de consultas só vale enquanto ninguém precisa decidir.
fn aguardar<B>(
    job: &mut Job<B>,
    espera: &Espera,
    dormir: &mut impl FnMut(Duration),
    etapa: Etapa,
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> Result<(), String>
where
    B: Backend + DescreveFalha,
    B::Erro: Display,
{
    let Etapa { passada, numero } = etapa;
    let mut restantes = espera.consultas;
    let mut erros_seguidos = 0;
    let mut motivo_respondido: Option<String> = None;
    while restantes > 0 {
        match job.atualizar() {
            Ok(EstadoJob::FrenteEnviada | EstadoJob::VersoEnviado) => {
                erros_seguidos = 0;
                match job.backend().motivo() {
                    Some(motivo) => {
                        if motivo_respondido.as_deref() != Some(motivo.as_str()) {
                            perguntar_no_aviso(job, etapa, &motivo, entrada, saida)?;
                            motivo_respondido = Some(motivo);
                        }
                    }
                    None => {
                        motivo_respondido = None;
                        restantes -= 1;
                    }
                }
                dormir(espera.intervalo);
            }
            Ok(_) => return Ok(()),
            Err(ErroJob::Backend(erro)) => {
                erros_seguidos += 1;
                if erros_seguidos >= ERROS_DE_CONSULTA_SEGUIDOS {
                    if job.backend().trabalho_saiu_da_fila() {
                        return Err(format!(
                            "não consegui confirmar {passada} do lote {numero}: {erro}. Não há o que cancelar na fila. Nada mais será enviado."
                        ));
                    }
                    return Err(format!(
                        "não consegui acompanhar {passada} do lote {numero}: {erro}. O trabalho não foi apagado; veja o estado com 'plots filas'. Nada mais será enviado."
                    ));
                }
                restantes -= 1;
                dormir(espera.intervalo);
            }
            Err(ErroJob::EnvioFalhou) => {
                return Err(falha_na_fila(
                    job,
                    etapa,
                    &ErroJob::EnvioFalhou,
                    entrada,
                    saida,
                ))
            }
            Err(erro) => return Err(texto_do_job(&erro)),
        }
    }
    Err(format!(
        "a fila não confirmou {passada} do lote {numero} a tempo. O trabalho continua na fila; veja o estado com 'plots filas'"
    ))
}

/// A impressora espera alguém: mostra o motivo e deixa o usuário escolher. `Ok` quer dizer que a
/// espera continua; `Err` traz o texto final, com o trabalho cancelado ou deixado na fila.
fn perguntar_no_aviso<B>(
    job: &mut Job<B>,
    etapa: Etapa,
    motivo: &str,
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> Result<(), String>
where
    B: Backend + DescreveFalha,
    B::Erro: Display,
{
    let Etapa { passada, numero } = etapa;
    writeln!(saida, "  Aviso sobre {passada} do lote {numero}: {motivo}.")
        .map_err(|e| e.to_string())?;
    // Sem resposta legível, nada é apagado: o trabalho fica na fila.
    let resposta = perguntar(entrada, saida, PERGUNTA_CANCELAR).unwrap_or(Resposta::FimDaEntrada);
    let desfecho = match resposta {
        Resposta::Nao => {
            writeln!(
                saida,
                "  Continuando a esperar o trabalho na fila. Ctrl+C encerra sem apagar o trabalho."
            )
            .map_err(|e| e.to_string())?;
            return saida.flush().map_err(|e| e.to_string());
        }
        Resposta::Sim => cancelar_na_fila(job),
        Resposta::FimDaEntrada => MSG_DEIXADO_NA_FILA.to_string(),
    };
    Err(format!("{passada} do lote {numero}: {motivo}. {desfecho}"))
}

/// A passada falhou na fila: mostra o motivo e só tira o trabalho de lá se o usuário pedir.
fn falha_na_fila<B>(
    job: &mut Job<B>,
    etapa: Etapa,
    erro: &ErroJob<B::Erro>,
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> String
where
    B: Backend + DescreveFalha,
    B::Erro: Display,
{
    let Etapa { passada, numero } = etapa;
    let motivo = job.backend().motivo().unwrap_or_else(|| texto_do_job(erro));
    let _ = writeln!(saida, "  {passada} do lote {numero} falhou: {motivo}.");
    // Sem resposta legível, nada é apagado: o trabalho fica na fila.
    let desfecho = match perguntar(entrada, saida, PERGUNTA_CANCELAR) {
        Ok(Resposta::Sim) => cancelar_na_fila(job),
        _ => MSG_DEIXADO_NA_FILA.to_string(),
    };
    format!("{passada} do lote {numero} falhou: {motivo}. {desfecho}")
}

fn cancelar_na_fila<B>(job: &mut Job<B>) -> String
where
    B: Backend,
    B::Erro: Display,
{
    match job.cancelar_envio() {
        Ok(()) => MSG_CANCELADO.to_string(),
        Err(falha) => format!(
            "Não foi possível cancelar o trabalho ({}); veja o estado com 'plots filas'. Nada mais será enviado.",
            texto_do_job(&falha)
        ),
    }
}

fn falha_ao_enviar<E: Display>(passada: &str, numero: usize, erro: &ErroJob<E>) -> String {
    format!(
        "não foi possível enviar {passada} do lote {numero}: {}",
        texto_do_job(erro)
    )
}

fn texto_do_job<E: Display>(erro: &ErroJob<E>) -> String {
    match erro {
        ErroJob::Backend(erro) => erro.to_string(),
        ErroJob::EnvioFalhou => "o spooler informou uma falha".to_string(),
        ErroJob::SemLotes => "o documento não tem lotes para enviar".to_string(),
        ErroJob::EstadoInvalido(estado) => {
            format!("operação fora de ordem no estado {estado:?}")
        }
    }
}

/// O que o usuário respondeu a uma pergunta de sim ou não.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Resposta {
    Sim,
    Nao,
    FimDaEntrada,
}

/// Pergunta antes de um envio. Só um sim vale: fim da entrada conta como não.
fn confirmar(
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
    pergunta: &str,
) -> Result<bool, String> {
    Ok(perguntar(entrada, saida, pergunta)? == Resposta::Sim)
}

/// Pergunta até receber s ou n, ou até a entrada terminar. Uma linha sem o fim de linha é uma
/// entrada que acabou no meio e não conta como resposta.
fn perguntar(
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
    pergunta: &str,
) -> Result<Resposta, String> {
    loop {
        write!(saida, "  {pergunta}").map_err(|e| e.to_string())?;
        saida.flush().map_err(|e| e.to_string())?;
        let mut resposta = String::new();
        let lidos = entrada
            .read_line(&mut resposta)
            .map_err(|e| format!("erro ao ler a confirmação: {e}"))?;
        if lidos == 0 || !resposta.ends_with('\n') {
            writeln!(saida).map_err(|e| e.to_string())?;
            return Ok(Resposta::FimDaEntrada);
        }
        match resposta.trim().to_lowercase().as_str() {
            "s" | "sim" => return Ok(Resposta::Sim),
            "n" | "nao" | "não" => return Ok(Resposta::Nao),
            _ => writeln!(saida, "  {MSG_RESPOSTA_INVALIDA}").map_err(|e| e.to_string())?,
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use plots_core::{plano_lotes_por_papel, DimensoesPagina, PerfilImpressora};
    use plots_print::job::{EstadoEnvio, Passada};
    use plots_windows::Estado as EstadoFila;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::io::Cursor;
    use std::rc::Rc;

    #[derive(Debug, Clone, PartialEq)]
    enum Evento {
        Enviou(Passada, Vec<Option<usize>>),
        Consultou,
        Cancelou,
    }

    /// O que uma consulta do backend falso responde.
    #[derive(Clone, Copy)]
    enum Leitura {
        Estado(EstadoEnvio),
        /// Continua em andamento, mas com algo a resolver na impressora.
        Aviso(&'static str),
        ErroDeConsulta,
        /// O trabalho saiu da fila, e o arquivo pedido não apareceu.
        SemArquivo,
    }
    use Leitura::{Aviso, ErroDeConsulta, Estado, SemArquivo};

    /// Backend que só registra o que recebeu: nenhum teste daqui alcança uma fila.
    struct Falso {
        eventos: Rc<RefCell<Vec<Evento>>>,
        respostas: VecDeque<Leitura>,
        erro_ao_enviar: Option<&'static str>,
        motivo: Option<String>,
        saiu_da_fila: bool,
    }

    impl Backend for Falso {
        type Erro = String;
        fn enviar(
            &mut self,
            passada: Passada,
            paginas: &[plots_core::Pagina],
        ) -> Result<(), String> {
            if let Some(erro) = self.erro_ao_enviar {
                return Err(erro.to_string());
            }
            self.eventos.borrow_mut().push(Evento::Enviou(
                passada,
                paginas.iter().map(|pagina| pagina.numero).collect(),
            ));
            Ok(())
        }
        fn consultar(&mut self) -> Result<EstadoEnvio, String> {
            self.eventos.borrow_mut().push(Evento::Consultou);
            let leitura = self
                .respostas
                .pop_front()
                .unwrap_or(Estado(EstadoEnvio::Concluido));
            self.motivo = None;
            match leitura {
                Estado(EstadoEnvio::Falhou) => {
                    self.motivo = Some("o trabalho deu erro".to_string());
                    Ok(EstadoEnvio::Falhou)
                }
                Estado(estado) => Ok(estado),
                Aviso(motivo) => {
                    self.motivo = Some(motivo.to_string());
                    Ok(EstadoEnvio::EmAndamento)
                }
                ErroDeConsulta => Err("a fila não respondeu".to_string()),
                SemArquivo => {
                    self.saiu_da_fila = true;
                    Err("o trabalho saiu da fila, mas o arquivo não apareceu".to_string())
                }
            }
        }
        fn cancelar(&mut self) -> Result<(), String> {
            self.eventos.borrow_mut().push(Evento::Cancelou);
            Ok(())
        }
    }

    impl DescreveFalha for Falso {
        fn motivo(&self) -> Option<String> {
            self.motivo.clone()
        }
        fn trabalho_saiu_da_fila(&self) -> bool {
            self.saiu_da_fila
        }
    }

    struct Rodada {
        resultado: Result<(), String>,
        saida: String,
        eventos: Vec<Evento>,
    }

    const A4: DimensoesPagina = DimensoesPagina::new(210.0, 297.0);
    const A4_DEITADO: DimensoesPagina = DimensoesPagina::new(297.0, 210.0);
    const A5: DimensoesPagina = DimensoesPagina::new(148.0, 210.0);

    fn rodar_com(
        paginas: usize,
        entrada: &str,
        respostas: &[Leitura],
        erro_ao_enviar: Option<&'static str>,
        consultas: u32,
    ) -> Rodada {
        rodar_documento(
            &vec![A4; paginas],
            entrada,
            respostas,
            erro_ao_enviar,
            consultas,
        )
    }

    fn rodar_documento(
        dimensoes: &[DimensoesPagina],
        entrada: &str,
        respostas: &[Leitura],
        erro_ao_enviar: Option<&'static str>,
        consultas: u32,
    ) -> Rodada {
        let eventos = Rc::new(RefCell::new(Vec::new()));
        let backend = Falso {
            eventos: Rc::clone(&eventos),
            respostas: respostas.iter().copied().collect(),
            erro_ao_enviar,
            motivo: None,
            saiu_da_fila: false,
        };
        let lotes = geracao::lotes_do_plano(&geracao::Plano {
            lotes: plano_lotes_por_papel(
                dimensoes,
                1,
                OrdemPassadas::default(),
                PerfilImpressora::default(),
            )
            .unwrap(),
            pares_mistos: Vec::new(),
        });
        let destino = Destino {
            fila: "Fila de Teste",
            gesto_texto: Some("Vire o maço."),
        };
        let espera = Espera {
            intervalo: Duration::ZERO,
            consultas,
        };
        let mut leitor = Cursor::new(entrada.as_bytes().to_vec());
        let mut saida = Vec::new();
        let resultado = conduzir(
            backend,
            lotes,
            &destino,
            &espera,
            |_| {},
            &mut leitor,
            &mut saida,
        );
        let eventos = eventos.borrow().clone();
        Rodada {
            resultado,
            saida: String::from_utf8(saida).unwrap(),
            eventos,
        }
    }

    fn rodar(paginas: usize, entrada: &str) -> Rodada {
        rodar_com(paginas, entrada, &[], None, 10)
    }

    fn envios(eventos: &[Evento]) -> Vec<Passada> {
        eventos
            .iter()
            .filter_map(|evento| match evento {
                Evento::Enviou(passada, _) => Some(*passada),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn dois_lotes_confirmados_enviam_frente_e_verso_nessa_ordem() {
        let rodada = rodar(4, "s\ns\ns\ns\n");
        rodada.resultado.unwrap();
        assert_eq!(
            envios(&rodada.eventos),
            [
                Passada::Frente,
                Passada::Verso,
                Passada::Frente,
                Passada::Verso
            ]
        );
        assert!(rodada
            .saida
            .contains("Enviar a frente do lote 1 para Fila de Teste? (s/n)"));
        assert!(rodada
            .saida
            .contains("Enviar o verso do lote 2 para Fila de Teste? (s/n)"));
        assert!(rodada.saida.contains("Vire o maço."));
    }

    #[test]
    fn saida_pede_para_esperar_a_folha_e_nunca_chama_de_impresso() {
        let rodada = rodar(2, "s\ns\n");
        rodada.resultado.unwrap();
        assert!(rodada
            .saida
            .contains("Espere a última folha sair antes de recolocar o maço."));
        let minusculas = rodada.saida.to_lowercase();
        let palavras: Vec<&str> = minusculas.split(|c: char| !c.is_alphabetic()).collect();
        for proibida in [
            "impresso",
            "impressos",
            "imprimiu",
            "imprimido",
            "impressão",
        ] {
            assert!(
                !palavras.contains(&proibida),
                "{proibida}: {}",
                rodada.saida
            );
        }
    }

    #[test]
    fn verso_so_e_oferecido_depois_de_a_fila_confirmar_a_frente() {
        let andamento = [Estado(EstadoEnvio::EmAndamento); 2];
        let rodada = rodar_com(2, "s\ns\n", &andamento, None, 10);
        rodada.resultado.unwrap();
        let frente = rodada
            .eventos
            .iter()
            .position(|e| matches!(e, Evento::Enviou(Passada::Frente, _)))
            .unwrap();
        let verso = rodada
            .eventos
            .iter()
            .position(|e| matches!(e, Evento::Enviou(Passada::Verso, _)))
            .unwrap();
        assert_eq!(verso - frente, 4, "{:?}", rodada.eventos);
        let confirmou = rodada
            .saida
            .find("O trabalho da frente do lote 1 terminou ou saiu da fila do spooler.")
            .unwrap();
        let pergunta = rodada.saida.find("Enviar o verso").unwrap();
        assert!(confirmou < pergunta);
    }

    #[test]
    fn fila_que_nunca_confirma_para_sem_oferecer_o_verso() {
        let andamento = [Estado(EstadoEnvio::EmAndamento); 5];
        let rodada = rodar_com(2, "s\ns\n", &andamento, None, 3);
        let erro = rodada.resultado.unwrap_err();
        assert!(erro.contains("não confirmou a frente"), "{erro}");
        assert_eq!(envios(&rodada.eventos), [Passada::Frente]);
        assert!(!rodada.saida.contains("Enviar o verso"));
        assert!(!rodada.eventos.contains(&Evento::Cancelou));
    }

    #[test]
    fn responder_nao_na_frente_cancela_sem_enviar_nada() {
        let rodada = rodar(2, "n\n");
        assert!(rodada.resultado.unwrap_err().contains("cancelado"));
        assert!(rodada.eventos.is_empty());
    }

    #[test]
    fn fim_da_entrada_na_frente_cancela_sem_enviar_nada() {
        for entrada in ["", "s"] {
            let rodada = rodar(2, entrada);
            assert!(rodada.resultado.unwrap_err().contains("cancelado"));
            assert!(rodada.eventos.is_empty(), "{entrada:?}");
        }
    }

    #[test]
    fn fim_da_entrada_no_verso_mantem_so_a_frente() {
        let rodada = rodar(2, "s\n");
        let erro = rodada.resultado.unwrap_err();
        assert!(erro.contains("a frente foi enviada, o verso não"), "{erro}");
        assert_eq!(envios(&rodada.eventos), [Passada::Frente]);
    }

    #[test]
    fn resposta_invalida_pergunta_de_novo_e_nao_envia() {
        let rodada = rodar(2, "talvez\n\nn\n");
        assert!(rodada.resultado.is_err());
        assert!(rodada.eventos.is_empty());
        assert_eq!(rodada.saida.matches("Enviar a frente").count(), 3);
        assert_eq!(rodada.saida.matches("Resposta inválida").count(), 2);
    }

    #[test]
    fn respostas_aceitam_maiusculas_e_a_palavra_inteira() {
        rodar(2, "S\nSim\n").resultado.unwrap();
    }

    const DEPOIS_DE_CANCELAR: &str = MSG_CANCELADO;

    #[test]
    fn texto_depois_de_cancelar_nao_promete_que_nada_mais_sai() {
        assert_eq!(
            MSG_CANCELADO,
            "O trabalho foi cancelado ou já tinha saído da fila; folhas já entregues ao dispositivo ainda podem sair. Nada mais será enviado."
        );
        assert!(PERGUNTA_CANCELAR.starts_with("Cancelar o trabalho na fila? (s/n)"));
    }

    #[test]
    fn cabecalho_diz_o_papel_e_a_troca_vem_antes_da_pergunta() {
        let dimensoes = [A4, A4, A5, A5];
        let rodada = rodar_documento(&dimensoes, "s\ns\ns\ns\n", &[], None, 10);
        rodada.resultado.unwrap();
        let saida = &rodada.saida;
        let cabecalho_1 = saida.find("Lote 1 de 2: 1 folha(s), papel A4").unwrap();
        let pergunta_1 = saida.find("Enviar a frente do lote 1").unwrap();
        assert!(cabecalho_1 < pergunta_1, "{saida}");
        let cabecalho_2 = saida.find("Lote 2 de 2: 1 folha(s), papel A5").unwrap();
        let troca = saida
            .find("Troque o papel para A5 e retire as folhas do lote anterior.")
            .unwrap();
        let pergunta_2 = saida.find("Enviar a frente do lote 2").unwrap();
        assert!(cabecalho_2 < troca && troca < pergunta_2, "{saida}");
        assert_eq!(saida.matches("Troque o papel").count(), 1, "{saida}");
    }

    #[test]
    fn mesmo_papel_nao_pede_troca_nem_quando_a_orientacao_muda() {
        let dimensoes = [A4, A4, A4_DEITADO, A4_DEITADO];
        let rodada = rodar_documento(&dimensoes, "s\ns\ns\ns\n", &[], None, 10);
        rodada.resultado.unwrap();
        assert!(!rodada.saida.contains("Troque o papel"), "{}", rodada.saida);
    }

    #[test]
    fn papel_fora_da_tabela_aparece_pelas_medidas_e_troca_quando_as_medidas_mudam() {
        let dimensoes = [
            DimensoesPagina::new(100.0, 150.0),
            DimensoesPagina::new(100.0, 150.0),
            DimensoesPagina::new(120.0, 170.0),
            DimensoesPagina::new(120.0, 170.0),
        ];
        let rodada = rodar_documento(&dimensoes, "s\ns\ns\ns\n", &[], None, 10);
        rodada.resultado.unwrap();
        assert!(
            rodada.saida.contains("papel 100 × 150 mm"),
            "{}",
            rodada.saida
        );
        assert!(
            rodada.saida.contains("Troque o papel para 120 × 170 mm"),
            "{}",
            rodada.saida
        );
    }

    fn fila(nome: &str, porta: &str) -> Fila {
        Fila {
            nome: nome.to_string(),
            driver: "Driver".to_string(),
            porta: porta.to_string(),
            padrao: false,
            estado: EstadoFila::de_bits(0),
        }
    }

    #[test]
    fn fila_inexistente_e_recusada_e_a_mensagem_aponta_a_listagem() {
        let filas = [fila("Fila A", "USB001")];
        let erro = decidir_fila(&filas, "Fila B", false).unwrap_err();
        assert!(erro.contains("não existe"), "{erro}");
        assert!(erro.contains("plots filas"), "{erro}");
        assert!(decidir_fila(&[], "Fila A", false).is_err());
    }

    #[test]
    fn nome_com_outra_caixa_ou_espaco_e_recusado() {
        let filas = [fila("Fila A", "USB001")];
        for nome in ["fila a", "FILA A", "Fila A ", " Fila A", "Fila  A"] {
            let erro = decidir_fila(&filas, nome, false).unwrap_err();
            assert!(erro.contains("não existe"), "{nome:?}: {erro}");
        }
    }

    #[test]
    fn porta_que_pede_arquivo_so_serve_com_pasta() {
        let filas = [fila("Virtual", "PORTPROMPT:")];
        let erro = decidir_fila(&filas, "Virtual", false).unwrap_err();
        assert!(erro.contains("--arquivo-saida"), "{erro}");
        assert_eq!(
            decidir_fila(&filas, "Virtual", true).unwrap().nome,
            "Virtual"
        );
    }

    #[test]
    fn fila_normal_serve_com_ou_sem_pasta_e_escolhe_o_nome_exato() {
        let filas = [fila("Fila", "USB001"), fila("Fila 2", "USB002")];
        for tem_pasta in [false, true] {
            let escolhida = decidir_fila(&filas, "Fila 2", tem_pasta).unwrap();
            assert_eq!(escolhida.porta, "USB002");
        }
    }

    #[test]
    fn fax_e_recusado_mesmo_com_pasta() {
        let filas = [fila("Fax", "SHRFAX:"), fila("Dois", "USB001, shrfax:")];
        for nome in ["Fax", "Dois"] {
            for tem_pasta in [false, true] {
                let erro = decidir_fila(&filas, nome, tem_pasta).unwrap_err();
                assert!(erro.contains("fax"), "{erro}");
            }
        }
    }

    #[test]
    fn falha_na_fila_mostra_o_motivo_pergunta_e_so_cancela_com_sim() {
        let rodada = rodar_com(2, "s\ns\n", &[Estado(EstadoEnvio::Falhou)], None, 10);
        let erro = rodada.resultado.unwrap_err();
        assert!(erro.contains("o trabalho deu erro"), "{erro}");
        assert!(erro.contains(DEPOIS_DE_CANCELAR), "{erro}");
        assert!(rodada.saida.contains(PERGUNTA_CANCELAR), "{}", rodada.saida);
        assert_eq!(envios(&rodada.eventos), [Passada::Frente]);
        assert!(rodada.eventos.contains(&Evento::Cancelou));
        assert!(!rodada.saida.contains("Enviar o verso"));
    }

    #[test]
    fn falha_na_fila_com_nao_ou_fim_da_entrada_deixa_o_trabalho_na_fila() {
        for entrada in ["s\nn\n", "s\n"] {
            let rodada = rodar_com(2, entrada, &[Estado(EstadoEnvio::Falhou)], None, 10);
            let erro = rodada.resultado.unwrap_err();
            assert!(rodada.saida.contains(PERGUNTA_CANCELAR), "{}", rodada.saida);
            assert!(erro.contains("O PlotS não apagou o trabalho"), "{erro}");
            assert!(erro.contains("plots filas"), "{erro}");
            assert!(erro.contains("Nada mais será enviado"), "{erro}");
            assert!(!rodada.eventos.contains(&Evento::Cancelou), "{entrada:?}");
            assert_eq!(envios(&rodada.eventos), [Passada::Frente]);
        }
    }

    #[test]
    fn aviso_com_nao_continua_esperando_e_nao_repete_a_pergunta_do_mesmo_motivo() {
        let leituras = [
            Aviso("a impressora está sem papel"),
            Aviso("a impressora está sem papel"),
            Estado(EstadoEnvio::EmAndamento),
            Estado(EstadoEnvio::Concluido),
        ];
        let rodada = rodar_com(2, "s\nn\ns\n", &leituras, None, 10);
        rodada.resultado.unwrap();
        assert_eq!(rodada.saida.matches(PERGUNTA_CANCELAR).count(), 1);
        assert!(
            rodada
                .saida
                .contains("Ctrl+C encerra sem apagar o trabalho."),
            "{}",
            rodada.saida
        );
        assert!(rodada.saida.contains("a impressora está sem papel"));
        assert!(!rodada.eventos.contains(&Evento::Cancelou));
        assert_eq!(envios(&rodada.eventos), [Passada::Frente, Passada::Verso]);
    }

    #[test]
    fn aviso_de_outro_motivo_pergunta_de_novo() {
        let leituras = [Aviso("sem papel"), Aviso("offline")];
        let rodada = rodar_com(2, "s\nn\nn\ns\n", &leituras, None, 10);
        rodada.resultado.unwrap();
        assert_eq!(rodada.saida.matches(PERGUNTA_CANCELAR).count(), 2);
    }

    #[test]
    fn aviso_com_sim_cancela_e_nao_oferece_o_verso() {
        let rodada = rodar_com(2, "s\ns\ns\n", &[Aviso("sem papel")], None, 10);
        let erro = rodada.resultado.unwrap_err();
        assert!(erro.contains("sem papel"), "{erro}");
        assert!(erro.contains(DEPOIS_DE_CANCELAR), "{erro}");
        assert!(rodada.eventos.contains(&Evento::Cancelou));
        assert!(!rodada.saida.contains("Enviar o verso"));
    }

    #[test]
    fn aviso_com_fim_da_entrada_deixa_o_trabalho_na_fila_e_diz_isso() {
        let rodada = rodar_com(2, "s\n", &[Aviso("sem papel")], None, 10);
        let erro = rodada.resultado.unwrap_err();
        assert!(erro.contains("sem papel"), "{erro}");
        assert!(
            erro.contains("O PlotS não apagou o trabalho; veja o estado com 'plots filas'. Nada mais será enviado."),
            "{erro}"
        );
        assert!(!rodada.eventos.contains(&Evento::Cancelou));
        assert!(!rodada.saida.contains("Enviar o verso"));
    }

    #[test]
    fn aviso_nao_se_esgota_no_limite_de_consultas() {
        let leituras = [Aviso("sem papel"); 8];
        let rodada = rodar_com(2, "s\nn\ns\n", &leituras, None, 3);
        rodada.resultado.unwrap();
    }

    #[test]
    fn erro_de_consulta_passageiro_e_tolerado_e_nao_cancela() {
        let leituras = vec![ErroDeConsulta; (ERROS_DE_CONSULTA_SEGUIDOS - 1) as usize];
        let rodada = rodar_com(2, "s\ns\n", &leituras, None, 1_000);
        rodada.resultado.unwrap();
        assert!(!rodada.eventos.contains(&Evento::Cancelou));
        assert_eq!(envios(&rodada.eventos), [Passada::Frente, Passada::Verso]);
    }

    #[test]
    fn contagem_de_erros_de_consulta_recomeca_depois_de_uma_resposta() {
        let mut leituras = vec![ErroDeConsulta; (ERROS_DE_CONSULTA_SEGUIDOS - 1) as usize];
        leituras.push(Estado(EstadoEnvio::EmAndamento));
        leituras.extend(vec![
            ErroDeConsulta;
            (ERROS_DE_CONSULTA_SEGUIDOS - 1) as usize
        ]);
        let rodada = rodar_com(2, "s\ns\n", &leituras, None, 1_000);
        rodada.resultado.unwrap();
    }

    #[test]
    fn erros_de_consulta_seguidos_param_sem_nunca_apagar_o_trabalho() {
        let leituras = vec![ErroDeConsulta; ERROS_DE_CONSULTA_SEGUIDOS as usize];
        let rodada = rodar_com(2, "s\ns\n", &leituras, None, 1_000);
        let erro = rodada.resultado.unwrap_err();
        assert!(erro.contains("a fila não respondeu"), "{erro}");
        assert!(erro.contains("não foi apagado"), "{erro}");
        assert!(!rodada.eventos.contains(&Evento::Cancelou));
        assert!(!rodada.saida.contains("Enviar o verso"));
    }

    #[test]
    fn arquivo_que_nao_apareceu_nao_fala_em_trabalho_que_ja_saiu_da_fila() {
        let leituras = vec![SemArquivo; ERROS_DE_CONSULTA_SEGUIDOS as usize];
        let rodada = rodar_com(2, "s\ns\n", &leituras, None, 1_000);
        let erro = rodada.resultado.unwrap_err();
        assert!(erro.contains("o arquivo não apareceu"), "{erro}");
        assert!(erro.contains("Não há o que cancelar na fila"), "{erro}");
        assert!(erro.contains("Nada mais será enviado"), "{erro}");
        assert!(!erro.contains("não foi apagado"), "{erro}");
        assert!(!erro.contains("plots filas"), "{erro}");
        assert!(!rodada.eventos.contains(&Evento::Cancelou));
    }

    #[test]
    fn textos_dizem_que_o_trabalho_saiu_da_fila_e_nao_que_o_papel_saiu() {
        let rodada = rodar(2, "s\ns\n");
        rodada.resultado.unwrap();
        for texto in [
            "O trabalho da frente do lote 1 terminou ou saiu da fila do spooler. Isso não garante que o papel já saiu.",
            "O trabalho do verso do lote 1 terminou ou saiu da fila do spooler.",
            "Os trabalhos dos 1 lote(s) terminaram ou saíram da fila do spooler. Confira as folhas depois que a última sair.",
        ] {
            assert!(rodada.saida.contains(texto), "{texto}: {}", rodada.saida);
        }
        assert!(!rodada.saida.contains("confirmou"), "{}", rodada.saida);
    }

    #[test]
    fn erro_ao_enviar_vira_mensagem_e_nao_segue_adiante() {
        let rodada = rodar_com(2, "s\ns\n", &[], Some("a biblioteca falhou"), 10);
        let erro = rodada.resultado.unwrap_err();
        assert!(erro.contains("a biblioteca falhou"), "{erro}");
        assert!(erro.contains("a frente do lote 1"), "{erro}");
        assert!(!rodada.saida.contains("Enviar o verso"));
    }

    #[test]
    fn passadas_seguem_a_ordem_do_plano_com_a_pagina_em_branco() {
        let rodada = rodar(3, "s\ns\ns\ns\n");
        rodada.resultado.unwrap();
        let enviadas: Vec<&Vec<Option<usize>>> = rodada
            .eventos
            .iter()
            .filter_map(|evento| match evento {
                Evento::Enviou(_, paginas) => Some(paginas),
                _ => None,
            })
            .collect();
        assert_eq!(
            enviadas,
            [&vec![Some(2)], &vec![Some(1)], &vec![None], &vec![Some(3)]]
        );
    }
}
