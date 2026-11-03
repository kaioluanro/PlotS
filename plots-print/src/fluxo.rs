//! Condução de um trabalho por impressora: lotes, frente, espera, recolocação, verso, avisos e
//! cancelamento. Não lê nem escreve em terminal ou janela: quem chama recebe cada passo e decide
//! como perguntar ao usuário. Nada aqui apaga um trabalho sem um pedido explícito.

use crate::job::{Backend, ErroJob, EstadoEnvio, EstadoJob, Job, Passada, SimBackend};
use crate::{conferir_folhas, ErroSimulacao, Mecanica, Recolocacao, Veredito};
use plots_core::{
    FormatoPapel, Lote, LotePapel, ModeloImpressora, Pagina, Papel, TOLERANCIA_PAPEL_MM,
};
use std::time::Duration;

pub const FRASE_ESPERAR_FOLHA: &str = "Espere a última folha sair antes de recolocar o maço.";
pub const FRASE_DA_CALIBRACAO: &str =
    "Pegue o maço como saiu, não embaralhe, e recoloque do mesmo jeito que na calibração.";
pub const FRASE_VIRAR_PARA_LER: &str = "Vire o maço antes de ler as folhas.";

/// Consultas com erro, em seguida, antes de desistir de acompanhar o trabalho.
pub const ERROS_DE_CONSULTA_SEGUIDOS: u32 = 30;

/// Meia hora de consultas, uma a cada segundo. A espera por alguém resolver um aviso da
/// impressora não conta.
pub const ESPERA_REAL: Espera = Espera {
    intervalo: Duration::from_secs(1),
    consultas: 1_800,
};

/// Com que frequência perguntar à fila e até quando esperar.
#[derive(Clone, Copy, Debug)]
pub struct Espera {
    pub intervalo: Duration,
    pub consultas: u32,
}

/// Condição da impressora que o backend distingue e que pede uma orientação própria. A orientação
/// é decidida por este valor, nunca por busca no texto do motivo.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CondicaoDoAviso {
    SemPapel,
    Atolado,
    SemResposta,
}

/// O que o backend sabe dizer sobre o trabalho, além do estado.
pub trait DescreveFalha {
    /// Por que o trabalho falhou ou o que a impressora espera que alguém resolva. Vazio quando o
    /// trabalho anda sem problema.
    fn motivo(&self) -> Option<String>;
    /// A condição mais urgente por trás do motivo, quando ela é uma das que o backend distingue.
    fn condicao(&self) -> Option<CondicaoDoAviso> {
        None
    }
    /// O que o próprio driver disse sobre o trabalho, já limpo. É entrada de terceiros: vem em campo
    /// próprio, nunca dentro do motivo, e não decide nada.
    fn mensagem_do_driver(&self) -> Option<String> {
        None
    }
    /// O trabalho já não está na fila, e por isso não há o que apagar nem cancelar.
    fn trabalho_saiu_da_fila(&self) -> bool;
    /// Folhas certas e total do lote recém-terminado, quando o backend consegue conferir.
    fn folhas_conferidas(&self, _paginas: &[Pagina]) -> Option<(usize, usize)> {
        None
    }
}

/// Um lote com o papel em que ele sai.
#[derive(Clone, Debug, PartialEq)]
pub struct LoteDoFluxo {
    pub papel: Papel,
    pub lote: Lote,
}

/// Lotes do plano do documento, cada um com o seu papel.
pub fn lotes_do_plano(lotes: &[LotePapel]) -> Vec<LoteDoFluxo> {
    lotes
        .iter()
        .map(|lote| LoteDoFluxo {
            papel: lote.papel,
            lote: Lote {
                paginas: lote
                    .paginas
                    .iter()
                    .map(|numero| Pagina {
                        numero: *numero,
                        girar_180: false,
                    })
                    .collect(),
                passada1: lote.passada1.clone(),
                passada2: lote.passada2.clone(),
            },
        })
        .collect()
}

/// Nome do papel como o usuário o procura na gaveta.
pub fn nome_do_papel(papel: &Papel) -> String {
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
pub fn mesma_folha(a: &Papel, b: &Papel) -> bool {
    a.formato == b.formato
        && (a.formato != FormatoPapel::Personalizado
            || ((a.largura_mm - b.largura_mm).abs() <= TOLERANCIA_PAPEL_MM
                && (a.altura_mm - b.altura_mm).abs() <= TOLERANCIA_PAPEL_MM))
}

/// Pares de páginas de cada folha do lote, na ordem de leitura.
pub fn pares_esperados(paginas: &[Pagina]) -> Vec<(Option<usize>, Option<usize>)> {
    paginas
        .chunks(2)
        .map(|par| {
            (
                par.first().and_then(|p| p.numero),
                par.get(1).and_then(|p| p.numero),
            )
        })
        .collect()
}

/// Lado para o qual a face impressa fica quando o maço entra de novo.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lado {
    Cima,
    Baixo,
}

/// Como o maço é movido antes de voltar à entrada.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Virada {
    PelaBordaDeCima,
    PelaLateral,
    NaoVirar,
    Girar,
}

/// As duas linhas da recolocação, em forma de dados, para cada interface pôr no seu idioma.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Instrucao {
    pub lado: Lado,
    pub virada: Virada,
}

impl Instrucao {
    pub fn do_gesto(mecanica: Mecanica, gesto: Recolocacao) -> Self {
        Self {
            lado: lado_na_entrada(mecanica, gesto),
            virada: virada_do_gesto(gesto),
        }
    }

    pub fn do_modelo(modelo: ModeloImpressora) -> Self {
        Self::do_gesto(Mecanica::do_modelo(modelo), modelo.gesto().into())
    }
}

/// A saída da impressora, invertida quando o gesto vira o maço. O simulador faz a mesma inversão
/// em `recolocar`.
fn lado_na_entrada(mecanica: Mecanica, gesto: Recolocacao) -> Lado {
    if mecanica.saida_face_impressa_para_cima != gesto.virar_maco {
        Lado::Cima
    } else {
        Lado::Baixo
    }
}

/// Os gestos dos modelos só viram, viram e giram, ou não tocam o maço.
fn virada_do_gesto(gesto: Recolocacao) -> Virada {
    match (gesto.virar_maco, gesto.girar_180) {
        (true, true) => Virada::PelaBordaDeCima,
        (true, false) => Virada::PelaLateral,
        (false, false) => Virada::NaoVirar,
        (false, true) => Virada::Girar,
    }
}

/// Frase de colocação: vem depois do gesto, e diz o lado conforme a saída da impressora.
pub fn gesto_de_recolocacao(mecanica: Mecanica, gesto: Recolocacao) -> String {
    let lado = match lado_na_entrada(mecanica, gesto) {
        Lado::Cima => "cima",
        Lado::Baixo => "baixo",
    };
    format!(
        "Depois, sem mudar a ordem das folhas, coloque na entrada com a face impressa para {lado}."
    )
}

/// Primeira linha da recolocação, dita pelo gesto: vire, gire ou não vire o maço antes de colocá-lo.
pub fn texto_do_gesto(gesto: Recolocacao) -> &'static str {
    match virada_do_gesto(gesto) {
        Virada::PelaBordaDeCima => "Vire o maço pela borda de cima, como um bloco de notas.",
        Virada::PelaLateral => "Vire o maço pela lateral, como a página de um livro.",
        Virada::NaoVirar => "Não vire o maço.",
        Virada::Girar => "Gire o maço 180°, sem virá-lo.",
    }
}

/// As linhas da recolocação, na ordem física: primeiro o gesto, depois a colocação na entrada.
pub fn linhas_de_recolocacao(mecanica: Mecanica, gesto: Recolocacao) -> [String; 2] {
    [
        texto_do_gesto(gesto).to_owned(),
        gesto_de_recolocacao(mecanica, gesto),
    ]
}

/// As duas linhas da recolocação de um modelo, na ordem física: o gesto e a colocação.
pub fn frases_de_recolocacao_do_modelo(modelo: ModeloImpressora) -> (&'static str, String) {
    let gesto = Recolocacao::from(modelo.gesto());
    (
        texto_do_gesto(gesto),
        gesto_de_recolocacao(Mecanica::do_modelo(modelo), gesto),
    )
}

/// Na saída com a face impressa para baixo, o maço final precisa ser virado para a leitura.
pub const fn vira_para_ler(modelo: ModeloImpressora) -> bool {
    !Mecanica::do_modelo(modelo).saida_face_impressa_para_cima
}

/// O que aconteceu em uma consulta à fila.
#[derive(Debug)]
pub enum Passo<E> {
    /// Nada a decidir: quem chama espera `intervalo` e consulta de novo.
    EmAndamento,
    /// A impressora espera alguém. `novo` só é verdadeiro na primeira vez de cada motivo: repetir
    /// a pergunta do mesmo motivo a cada consulta só atrapalharia. A mensagem do driver pode mudar
    /// sem que o motivo mude, e isso não reabre a pergunta.
    Aviso {
        motivo: String,
        condicao: Option<CondicaoDoAviso>,
        mensagem_do_driver: Option<String>,
        novo: bool,
    },
    /// A fila confirmou a frente: agora vale a recolocação e o verso.
    FrenteConfirmada,
    /// O verso terminou e ainda há lotes.
    LoteConcluido,
    /// O verso do último lote terminou.
    DocumentoConcluido,
    /// A passada falhou na fila; o motivo vem do backend, quando ele sabe dizer.
    Falhou { motivo: Option<String> },
    /// A consulta errou seguidas vezes. O trabalho não foi tocado.
    SemAcompanhamento { erro: E, saiu_da_fila: bool },
    /// A fila não confirmou dentro do limite de consultas. O trabalho continua lá.
    TempoEsgotado,
    /// Consulta sem passada na fila.
    ForaDeOrdem(ErroJob<E>),
}

/// Conduz um documento em uma impressora, um passo de cada vez.
pub struct Condutor<B: Backend> {
    job: Job<B>,
    lotes: Vec<LoteDoFluxo>,
    espera: Espera,
    restantes: u32,
    erros_seguidos: u32,
    motivo_apresentado: Option<String>,
}

impl<B: Backend + DescreveFalha> Condutor<B> {
    pub fn novo(
        backend: B,
        lotes: Vec<LoteDoFluxo>,
        espera: Espera,
    ) -> Result<Self, ErroJob<B::Erro>> {
        let para_o_job = lotes.iter().map(|item| item.lote.clone()).collect();
        Ok(Self {
            job: Job::novo(backend, para_o_job)?,
            lotes,
            espera,
            restantes: espera.consultas,
            erros_seguidos: 0,
            motivo_apresentado: None,
        })
    }

    pub fn estado(&self) -> EstadoJob {
        self.job.estado()
    }

    pub fn backend(&self) -> &B {
        self.job.backend()
    }

    pub fn intervalo(&self) -> Duration {
        self.espera.intervalo
    }

    pub fn total(&self) -> usize {
        self.lotes.len()
    }

    /// Número do lote atual, começando em 1. Depois do último, continua nele.
    pub fn numero(&self) -> usize {
        self.job.lote_atual().saturating_add(1).min(self.total())
    }

    /// O lote atual; ao fim do documento, o último.
    pub fn lote(&self) -> Option<&LoteDoFluxo> {
        self.lotes.get(self.numero().saturating_sub(1))
    }

    /// Lote que acabou de terminar: o anterior ao atual, ou o último ao fim do documento.
    pub fn lote_terminado(&self) -> Option<&LoteDoFluxo> {
        let indice = self.job.lote_atual().checked_sub(1)?;
        self.lotes.get(indice)
    }

    /// Papel a colocar antes do lote atual, quando ele é outro que o do lote anterior.
    pub fn troca_de_papel(&self) -> Option<Papel> {
        let atual = self.lotes.get(self.job.lote_atual())?;
        let anterior = self.lotes.get(self.job.lote_atual().checked_sub(1)?)?;
        (!mesma_folha(&anterior.papel, &atual.papel)).then_some(atual.papel)
    }

    fn rearmar(&mut self) {
        self.restantes = self.espera.consultas;
        self.erros_seguidos = 0;
        self.motivo_apresentado = None;
    }

    pub fn enviar_frente(&mut self) -> Result<(), ErroJob<B::Erro>> {
        self.rearmar();
        self.job.enviar_frente()
    }

    /// Só vale depois de a fila confirmar a frente: fora disso, nada é enviado.
    pub fn enviar_verso(&mut self) -> Result<(), ErroJob<B::Erro>> {
        self.rearmar();
        self.job.enviar_verso()
    }

    /// O limite de consultas volta a valer do início, sem tocar no trabalho.
    pub fn continuar_esperando(&mut self) {
        self.restantes = self.espera.consultas;
    }

    /// Consulta a fila uma vez. Um trabalho só sai da fila por decisão do usuário: erro ao
    /// consultar não diz nada sobre ele, então nunca leva a cancelar. O limite de consultas só
    /// conta enquanto ninguém precisa decidir.
    pub fn consultar(&mut self) -> Passo<B::Erro> {
        if self.restantes == 0 {
            return Passo::TempoEsgotado;
        }
        match self.job.atualizar() {
            Ok(EstadoJob::FrenteEnviada | EstadoJob::VersoEnviado) => {
                self.erros_seguidos = 0;
                match self.job.backend().motivo() {
                    Some(motivo) => {
                        let novo = self.motivo_apresentado.as_deref() != Some(motivo.as_str());
                        self.motivo_apresentado = Some(motivo.clone());
                        Passo::Aviso {
                            motivo,
                            condicao: self.job.backend().condicao(),
                            mensagem_do_driver: self.job.backend().mensagem_do_driver(),
                            novo,
                        }
                    }
                    None => {
                        self.motivo_apresentado = None;
                        self.restantes = self.restantes.saturating_sub(1);
                        Passo::EmAndamento
                    }
                }
            }
            Ok(EstadoJob::AguardandoRecolocacao) => Passo::FrenteConfirmada,
            Ok(EstadoJob::Pronto) => Passo::LoteConcluido,
            Ok(EstadoJob::Concluido) => Passo::DocumentoConcluido,
            Ok(EstadoJob::Erro) | Err(ErroJob::EnvioFalhou) => Passo::Falhou {
                motivo: self.job.backend().motivo(),
            },
            Err(ErroJob::Backend(erro)) => {
                self.erros_seguidos = self.erros_seguidos.saturating_add(1);
                if self.erros_seguidos >= ERROS_DE_CONSULTA_SEGUIDOS {
                    return Passo::SemAcompanhamento {
                        erro,
                        saiu_da_fila: self.job.backend().trabalho_saiu_da_fila(),
                    };
                }
                self.restantes = self.restantes.saturating_sub(1);
                Passo::EmAndamento
            }
            Err(outro) => Passo::ForaDeOrdem(outro),
        }
    }

    /// Tira da fila o trabalho da passada atual, ou o que falhou. Quem decide é o usuário.
    pub fn cancelar(&mut self) -> Result<(), ErroJob<B::Erro>> {
        self.rearmar();
        self.job.cancelar_envio()
    }

    /// Refaz só o lote atual, desde a frente. Só vale com a passada falha ou ainda na fila.
    pub fn reimprimir_lote(&mut self) -> Result<(), ErroJob<B::Erro>> {
        self.cancelar()?;
        self.enviar_frente()
    }
}

impl DescreveFalha for SimBackend {
    fn motivo(&self) -> Option<String> {
        None
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        false
    }
}

/// Backend da simulação pela janela: aplica o gesto do modelo antes do verso, como faria quem
/// segue a instrução, e confere as folhas ao fim de cada lote. Nada vai a uma fila.
pub struct SimGuiado {
    sim: SimBackend,
    gesto: Recolocacao,
}

impl SimGuiado {
    pub fn novo(mecanica: Mecanica, gesto: Recolocacao) -> Self {
        Self {
            sim: SimBackend::novo(mecanica),
            gesto,
        }
    }
}

impl Backend for SimGuiado {
    type Erro = ErroSimulacao;
    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), ErroSimulacao> {
        if passada == Passada::Verso {
            self.sim.recolocar(self.gesto)?;
        }
        self.sim.enviar(passada, paginas)
    }
    fn consultar(&mut self) -> Result<EstadoEnvio, ErroSimulacao> {
        self.sim.consultar()
    }
    fn cancelar(&mut self) -> Result<(), ErroSimulacao> {
        self.sim.cancelar()
    }
}

impl DescreveFalha for SimGuiado {
    fn motivo(&self) -> Option<String> {
        None
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        false
    }
    fn folhas_conferidas(&self, paginas: &[Pagina]) -> Option<(usize, usize)> {
        let pilha = self.sim.simulador().folhas_para_leitura();
        let vereditos = conferir_folhas(&pilha, &pares_esperados(paginas));
        let corretas = vereditos
            .iter()
            .filter(|veredito| matches!(veredito, Veredito::Certa))
            .count();
        Some((corretas, vereditos.len()))
    }
}

#[cfg(test)]
#[path = "fluxo_testes.rs"]
mod testes;
