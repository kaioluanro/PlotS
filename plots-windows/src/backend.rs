//! Envio das passadas ao spooler do Windows.
//!
//! Tudo aqui bloqueia enquanto a biblioteca desenha e o driver recebe as páginas: quem tiver uma
//! interface deve chamar a partir de outra thread.

use crate::erro::ErroEnvio;
use crate::estado::Condicao;
use crate::papel::{ajuste_do_papel, FacesDoEnvio};
use crate::passada::{
    caminhos_da_biblioteca, escolher_biblioteca, nome_arquivo_saida, nome_documento,
    papel_da_passada, VARIAVEL_PASTA_BIBLIOTECA,
};
use crate::preferencias::{ajuste_salvo, avisar_recusa, fila_declara_duas_faces};
use crate::preferencias_driver::com_recuo_ao_padrao;
use crate::raster::{
    conferir_papel, fator_de_unidade, marcar_ponto_discreto, planejar_pagina,
    ponto_da_pagina_em_branco, Dispositivo, PlanoPagina,
};
use crate::spooler::{ContextoFila, DocumentoAberto, Impressora};
use crate::trabalho::{motivo_e_mensagem, trabalho_terminou, traduzir_com_fila};
use crate::vez::{driver_pede_uma_fila_por_vez, Posse, NOME_DA_VEZ_DO_PDF, PRAZO_DA_VEZ};
use crate::Fila;
use pdfium_render::prelude::{
    PdfBitmapFormat, PdfColor, PdfDocument, PdfPageRenderRotation, PdfRenderConfig, Pdfium,
    PdfiumError,
};
use plots_core::{DimensoesPagina, IdentidadeDaPagina, Pagina};
use plots_pdf::{OpcoesDaPassada, PdfOrigem};
use plots_print::fluxo::{CondicaoDoAviso, DescreveFalha};
use plots_print::job::{Backend, EstadoEnvio, Passada};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// A biblioteca só pode ser iniciada uma vez por processo.
static BIBLIOTECA: OnceLock<Pdfium> = OnceLock::new();

/// Carrega a biblioteca só pelo caminho explícito: a pasta indicada ou ao lado do executável.
///
/// A análise de tinta usa a mesma instância: duas inicializações no mesmo processo são recusadas
/// pela própria biblioteca.
pub(crate) fn biblioteca() -> Result<&'static Pdfium, ErroEnvio> {
    if let Some(pronta) = BIBLIOTECA.get() {
        return Ok(pronta);
    }
    let indicada = std::env::var_os(VARIAVEL_PASTA_BIBLIOTECA);
    let executavel = std::env::current_exe().ok();
    let pasta_executavel = executavel.as_deref().and_then(Path::parent);
    let caminhos = caminhos_da_biblioteca(indicada.as_deref(), pasta_executavel)?;
    let caminho = escolher_biblioteca(caminhos, Path::is_file)?;
    let ligacoes = Pdfium::bind_to_library(&caminho)
        .map_err(|erro| ErroEnvio::BibliotecaInvalida(erro.to_string()))?;
    Ok(BIBLIOTECA.get_or_init(|| Pdfium::new(ligacoes)))
}

fn erro_de_desenho(erro: PdfiumError) -> ErroEnvio {
    ErroEnvio::Rasterizacao(erro.to_string())
}

/// Tudo que uma passada precisa no spooler. `nome_fila` é usado sem nenhuma troca: esta camada
/// não tem fila preferida e envia só para a que recebeu.
pub struct BackendWindows {
    pdfium: &'static Pdfium,
    nome_fila: String,
    /// Porta da fila, que decide o nome do arquivo de saída.
    porta: String,
    impressora: Impressora,
    origem: PdfOrigem,
    opcoes_pdf: OpcoesDaPassada,
    marcas_dos_lotes: Vec<BTreeMap<IdentidadeDaPagina, String>>,
    dimensoes: Vec<DimensoesPagina>,
    pasta_saida: Option<PathBuf>,
    /// Lote em andamento, começando em 1 depois da primeira frente.
    lote: usize,
    /// Depois de um cancelamento, a próxima frente repete o mesmo lote.
    repetir_lote: bool,
    trabalho: Option<u32>,
    arquivo: Option<PathBuf>,
    ultimo_status: Option<u32>,
    motivo: Option<String>,
    condicao: Option<CondicaoDoAviso>,
    mensagem_do_driver: Option<String>,
    /// O trabalho atual foi visto fora da fila.
    saiu_da_fila: bool,
    /// Quando o envio atual começou, para procurar no registro do Windows a falha dele.
    enviado_em: Option<std::time::Instant>,
    /// O nome do documento do trabalho atual na fila, que o evento do registro também traz.
    nome_do_trabalho: Option<String>,
    /// Nos testes locais, este driver falhou com dois trabalhos juntos: um de cada vez, entre todos os processos.
    uma_fila_por_vez: bool,
    /// A vez de enviar, até o trabalho sair da fila.
    vez: Option<Posse>,
    /// Ajuste do driver lido ao preparar: mudar a preferência não atinge o trabalho em andamento.
    ajuste_do_driver: Option<Vec<u8>>,
}

impl BackendWindows {
    /// Prepara o envio para `fila`. Com `pasta_saida`, cada passada é gravada em um arquivo
    /// dentro dela pela própria fila. Carrega a biblioteca e abre a fila já aqui, para os erros
    /// aparecerem antes de qualquer pergunta. Nada é enviado.
    pub fn novo(
        fila: &Fila,
        origem: PdfOrigem,
        dimensoes: Vec<DimensoesPagina>,
        pasta_saida: Option<&Path>,
    ) -> Result<Self, ErroEnvio> {
        let pasta_saida = pasta_saida
            .map(|pasta| {
                if !pasta.is_dir() {
                    return Err(ErroEnvio::PastaDeSaidaInvalida(pasta.to_path_buf()));
                }
                // O caminho precisa ser absoluto: o spooler não conhece a pasta atual.
                std::path::absolute(pasta)
                    .map_err(|_| ErroEnvio::PastaDeSaidaInvalida(pasta.to_path_buf()))
            })
            .transpose()?;
        let pdfium = biblioteca()?;
        let impressora = Impressora::abrir(&fila.nome)?;
        let ajuste_do_driver = ajuste_salvo(&fila.nome);
        Ok(Self {
            pdfium,
            nome_fila: fila.nome.clone(),
            porta: fila.porta.clone(),
            impressora,
            origem,
            opcoes_pdf: OpcoesDaPassada::default(),
            marcas_dos_lotes: Vec::new(),
            dimensoes,
            pasta_saida,
            lote: 0,
            repetir_lote: false,
            trabalho: None,
            arquivo: None,
            ultimo_status: None,
            motivo: None,
            condicao: None,
            mensagem_do_driver: None,
            saiu_da_fila: false,
            enviado_em: None,
            nome_do_trabalho: None,
            uma_fila_por_vez: driver_pede_uma_fila_por_vez(&fila.driver),
            vez: None,
            ajuste_do_driver,
        })
    }

    /// Define o papel da saída antes de iniciar o trabalho.
    pub fn definir_papel_de_saida(&mut self, papel_mm: Option<(f64, f64)>) {
        self.opcoes_pdf.papel_mm = papel_mm;
    }

    /// O texto a escrever nos versos de cada lote deste trabalho, na ordem dos lotes.
    pub fn definir_marcas(&mut self, marcas: Vec<BTreeMap<IdentidadeDaPagina, String>>) {
        self.marcas_dos_lotes = marcas;
    }

    /// As opções da passada do lote atual: o papel é o de sempre, e a marca segue a página.
    fn opcoes_da_passada(&self) -> OpcoesDaPassada {
        let marcas = self
            .lote
            .checked_sub(1)
            .and_then(|indice| self.marcas_dos_lotes.get(indice))
            .cloned()
            .unwrap_or_default();
        OpcoesDaPassada {
            papel_mm: self.opcoes_pdf.papel_mm,
            marcas,
        }
    }

    /// Último estado bruto lido do spooler; `None` antes da primeira leitura ou depois que o
    /// trabalho saiu da fila.
    pub fn ultimo_status(&self) -> Option<u32> {
        self.ultimo_status
    }

    /// O trabalho atual já não está na fila: não há mais o que apagar nem cancelar.
    pub fn saiu_da_fila(&self) -> bool {
        self.saiu_da_fila
    }

    /// Por que o trabalho atual falhou ou o que a impressora espera que alguém resolva.
    pub fn motivo_atual(&self) -> Option<&str> {
        self.motivo.as_deref()
    }

    fn proximo_arquivo(&self, passada: Passada) -> Result<Option<PathBuf>, ErroEnvio> {
        let Some(pasta) = &self.pasta_saida else {
            return Ok(None);
        };
        let arquivo = pasta.join(nome_arquivo_saida(self.lote, passada, &self.porta));
        if arquivo.exists() {
            return Err(ErroEnvio::ArquivoJaExiste(arquivo));
        }
        Ok(Some(arquivo))
    }
}

/// O que dizer quando o Windows descartou o trabalho. A interface traduz esta frase.
pub const MOTIVO_TRABALHO_PERDIDO: &str =
    "o registro do Windows indica que a impressão falhou nesta fila. Confira o que saiu antes de reimprimir";

/// Planeja todas as páginas antes de abrir o documento: um tamanho recusado, ou uma página em
/// branco que não teria onde receber o ponto, não deixa trabalho pela metade na fila.
fn planejar_paginas(
    documento: &PdfDocument<'_>,
    paginas: &[Pagina],
    dimensoes: &[DimensoesPagina],
    dispositivo: &Dispositivo,
) -> Result<Vec<PlanoPagina>, ErroEnvio> {
    documento
        .pages()
        .iter()
        .zip(paginas)
        .map(|(desenho, pagina)| {
            let numero = pagina.numero.unwrap_or(dimensoes.len());
            let medida = numero
                .checked_sub(1)
                .and_then(|indice| dimensoes.get(indice))
                .ok_or(ErroEnvio::PaginaInexistente(numero))?;
            let caixa = desenho
                .boundaries()
                .media()
                .map_err(erro_de_desenho)?
                .bounds;
            let fator = fator_de_unidade(
                *medida,
                f64::from(caixa.width().value),
                f64::from(caixa.height().value),
            )?;
            let plano = planejar_pagina(
                f64::from(desenho.width().value) * fator,
                f64::from(desenho.height().value) * fator,
                dispositivo,
            )?;
            if pagina.numero.is_none() {
                ponto_da_pagina_em_branco(&plano, dispositivo)?;
            }
            Ok(plano)
        })
        .collect()
}

/// Rotação do bitmap: a página já traz a própria, e a da passada. Aqui só entra o giro para a
/// página deitada caber no papel em pé.
fn rotacao_do_bitmap(plano: &PlanoPagina) -> PdfPageRenderRotation {
    if plano.girar_90 {
        PdfPageRenderRotation::Degrees270
    } else {
        PdfPageRenderRotation::None
    }
}

/// Desenha as páginas uma de cada vez, sobre branco, em qualidade de impressão. Os pixels de
/// uma página são liberados antes de a próxima começar.
fn desenhar_paginas(
    documento: &PdfDocument<'_>,
    paginas: &[Pagina],
    planos: &[PlanoPagina],
    dispositivo: &Dispositivo,
    aberto: &mut DocumentoAberto<'_>,
) -> Result<(), ErroEnvio> {
    for (indice, (pagina, plano)) in paginas.iter().zip(planos).enumerate() {
        let pixels = rasterizar(documento, indice, *pagina, plano, dispositivo)?;
        aberto.desenhar_pagina(plano, &pixels)?;
    }
    Ok(())
}

/// Desenha uma página. A página em branco recebe o ponto já no bitmap, dentro da área
/// imprimível, para o driver não a descartar como vazia.
fn rasterizar(
    documento: &PdfDocument<'_>,
    indice: usize,
    origem: Pagina,
    plano: &PlanoPagina,
    dispositivo: &Dispositivo,
) -> Result<Vec<u8>, ErroEnvio> {
    let indice = i32::try_from(indice)
        .map_err(|_| ErroEnvio::Rasterizacao("passada com páginas demais".to_string()))?;
    let pagina = documento.pages().get(indice).map_err(erro_de_desenho)?;
    let configuracao = PdfRenderConfig::new()
        .set_fixed_size(plano.largura_px, plano.altura_px)
        .rotate(rotacao_do_bitmap(plano), false)
        .use_print_quality(true)
        .set_format(PdfBitmapFormat::BGRA)
        // O DIB do GDI é BGRA; a inversão que o pdfium-render liga por padrão entregaria RGBA.
        .set_reverse_byte_order(false)
        .clear_before_rendering(true)
        .set_clear_color(PdfColor::WHITE);
    let bitmap = pagina
        .render_with_config(&configuracao)
        .map_err(erro_de_desenho)?;
    let mut pixels = bitmap.as_raw_bytes();
    if origem.numero.is_none() {
        marcar_ponto_discreto(&mut pixels, plano, dispositivo)?;
    }
    Ok(pixels)
}

impl DescreveFalha for BackendWindows {
    fn motivo(&self) -> Option<String> {
        self.motivo_atual().map(str::to_string)
    }
    fn condicao(&self) -> Option<CondicaoDoAviso> {
        self.condicao
    }
    fn mensagem_do_driver(&self) -> Option<String> {
        self.mensagem_do_driver.clone()
    }
    fn trabalho_saiu_da_fila(&self) -> bool {
        self.saiu_da_fila()
    }
}

impl BackendWindows {
    fn enviar_passada(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), ErroEnvio> {
        if passada != Passada::Verso && !self.repetir_lote {
            self.lote = self.lote.saturating_add(1);
        }
        self.repetir_lote = false;
        self.trabalho = None;
        self.arquivo = None;
        self.ultimo_status = None;
        self.motivo = None;
        self.condicao = None;
        self.mensagem_do_driver = None;
        self.saiu_da_fila = false;
        self.enviado_em = None;
        self.nome_do_trabalho = None;
        self.vez = None;

        let faces = FacesDoEnvio::da_passada(passada);
        // O perfil pode ter sido gravado com outro driver: sem a declaração dele, nada sai.
        if faces == FacesDoEnvio::Duas
            && fila_declara_duas_faces(&self.nome_fila, &self.porta) != Some(true)
        {
            return Err(ErroEnvio::DuasFacesNaoAceitas);
        }
        let papel = papel_da_passada(paginas, &self.dimensoes)?;
        let ajuste = ajuste_do_papel(&papel)?;
        let arquivo = self.proximo_arquivo(passada)?;
        // A barreira de PDF protegido já passou em `PdfOrigem::carregar`; a biblioteca só vê o
        // PDF da passada, sem senha, com a ordem, as páginas em branco e os giros do plano.
        let pdf = self
            .origem
            .gerar_passada_com(paginas, self.dimensoes.len(), &self.opcoes_da_passada())
            .map_err(|erro| ErroEnvio::Pdf(erro.to_string()))?;
        let documento = self
            .pdfium
            .load_pdf_from_byte_slice(&pdf, None)
            .map_err(erro_de_desenho)?;
        if usize::try_from(documento.pages().len()).ok() != Some(paginas.len()) {
            return Err(ErroEnvio::Rasterizacao(
                "a passada abriu com outra quantidade de páginas".to_string(),
            ));
        }

        let (impressora, nome_fila) = (&self.impressora, &self.nome_fila);
        let ((contexto, dispositivo), recusado) =
            com_recuo_ao_padrao(&mut self.ajuste_do_driver, |salvo| {
                let devmode = impressora.devmode(ajuste, faces, salvo)?;
                let contexto = ContextoFila::criar(nome_fila, &devmode)?;
                let dispositivo = contexto.dispositivo();
                conferir_papel(&dispositivo, &papel)?;
                Ok((contexto, dispositivo))
            })?;
        if let Some(recusado) = recusado {
            avisar_recusa(&self.nome_fila, &recusado);
        }
        let planos = planejar_paginas(&documento, paginas, &self.dimensoes, &dispositivo)?;

        // Só neste ponto: a vez só vale enquanto o trabalho está na fila, e o preparo acima não a segura.
        let vez = if self.uma_fila_por_vez {
            Posse::pegar(NOME_DA_VEZ_DO_PDF, PRAZO_DA_VEZ)
        } else {
            None
        };
        let nome = nome_documento(self.lote, passada);
        self.enviado_em = Some(std::time::Instant::now());
        self.nome_do_trabalho = Some(nome.clone());
        let mut aberto = contexto.iniciar_documento(&nome, arquivo.as_deref())?;
        desenhar_paginas(&documento, paginas, &planos, &dispositivo, &mut aberto)?;
        let trabalho = aberto.terminar()?;
        self.trabalho = Some(trabalho);
        self.arquivo = arquivo;
        self.vez = vez;
        Ok(())
    }
}

/// Uma chamada do sistema que falha no meio do envio pode ser só a fila que saiu da lista: então a
/// causa é essa, e o código cru do Windows não ajuda ninguém. `fila_listada` só roda nesse caso.
fn explicar_falha_do_sistema(erro: ErroEnvio, fila_listada: impl FnOnce() -> bool) -> ErroEnvio {
    match erro {
        ErroEnvio::Sistema { .. } if !fila_listada() => ErroEnvio::FilaSaiuDaLista,
        outro => outro,
    }
}

/// A fila continua na lista do Windows. O sistema só tira da lista a fila removida no meio de um
/// envio depois que ele solta o trabalho, um ou dois segundos depois da falha: por isso se olha a
/// lista por alguns instantes antes de concluir que ela ficou. Sem conseguir listar, não se afirma
/// nada.
fn fila_esta_listada(nome: &str) -> bool {
    for tentativa in 0..TENTATIVAS_DE_CONFERIR_A_LISTA {
        if tentativa > 0 {
            std::thread::sleep(INTERVALO_DA_CONFERENCIA);
        }
        match crate::listar_filas() {
            Ok(filas) if !fila_na_lista(&filas, nome) => return false,
            Err(_) => return true,
            Ok(_) => {}
        }
    }
    true
}

/// Até 5 s olhando a lista, só depois de uma falha do sistema no meio do envio.
const TENTATIVAS_DE_CONFERIR_A_LISTA: u32 = 20;
const INTERVALO_DA_CONFERENCIA: std::time::Duration = std::time::Duration::from_millis(250);

/// Uma fila com exclusão pendente ainda aparece na lista enquanto o trabalho dela não acaba, mas já
/// saiu: o sistema recusa as chamadas dela como se não existisse.
fn fila_na_lista(filas: &[Fila], nome: &str) -> bool {
    filas.iter().any(|fila| {
        fila.nome == nome
            && !fila
                .estado
                .condicoes()
                .contains(&Condicao::ExclusaoPendente)
    })
}

impl Backend for BackendWindows {
    type Erro = ErroEnvio;

    fn enviar(&mut self, passada: Passada, paginas: &[Pagina]) -> Result<(), ErroEnvio> {
        let resultado = self.enviar_passada(passada, paginas);
        let nome = &self.nome_fila;
        resultado.map_err(|erro| explicar_falha_do_sistema(erro, || fila_esta_listada(nome)))
    }

    fn consultar(&mut self) -> Result<EstadoEnvio, ErroEnvio> {
        let trabalho = self.trabalho.ok_or(ErroEnvio::SemTrabalho)?;
        let Some(leitura) = self.impressora.ler_trabalho(trabalho)? else {
            // O trabalho sumiu da fila: o sistema não guarda como terminou. Este é um limite
            // conhecido: conta como terminado, salvo se o arquivo pedido não apareceu.
            self.ultimo_status = None;
            self.saiu_da_fila = true;
            self.vez = None;
            // O processador de impressão pode descartar o trabalho sem imprimir nada e só deixar o
            // evento 372 no registro: sumir da fila não prova que saiu papel.
            if let (Some(desde), Some(documento)) =
                (self.enviado_em, self.nome_do_trabalho.as_deref())
            {
                if crate::registro::trabalho_perdido(&self.nome_fila, documento, desde) {
                    self.motivo = Some(MOTIVO_TRABALHO_PERDIDO.to_string());
                    return Ok(EstadoEnvio::Falhou);
                }
            }
            if let Some(arquivo) = &self.arquivo {
                if !arquivo.is_file() {
                    return Err(ErroEnvio::ArquivoNaoGerado(arquivo.clone()));
                }
            }
            return Ok(EstadoEnvio::Concluido);
        };
        self.ultimo_status = Some(leitura.status);
        let fila = if trabalho_terminou(leitura.status) {
            self.vez = None;
            None
        } else {
            self.impressora.ler_estado()
        };
        let situacao = traduzir_com_fila(leitura.status, fila.as_ref());
        self.condicao = situacao.condicao;
        let (motivo, mensagem) =
            motivo_e_mensagem(situacao.estado, situacao.motivo, &leitura.texto);
        self.motivo = motivo;
        self.mensagem_do_driver = mensagem;
        Ok(situacao.estado)
    }

    fn cancelar(&mut self) -> Result<(), ErroEnvio> {
        self.repetir_lote = true;
        self.vez = None;
        cancelar_trabalho(&mut self.trabalho, |id| self.impressora.cancelar(id))
    }

    /// As frentes saíram em outro trabalho: o nome do documento e do arquivo segue o lote do maço.
    fn maco_de_fora(&mut self, indice: usize, _frentes: &[Pagina]) -> Result<(), ErroEnvio> {
        self.lote = indice.saturating_add(1);
        Ok(())
    }
}

/// Pede o cancelamento do trabalho guardado. O id só sai depois que a fila confirma: se o
/// cancelamento falhar, a próxima tentativa ainda tem como alcançar o trabalho.
fn cancelar_trabalho<F>(trabalho: &mut Option<u32>, cancelar: F) -> Result<(), ErroEnvio>
where
    F: FnOnce(u32) -> Result<(), ErroEnvio>,
{
    let Some(id) = *trabalho else {
        return Ok(());
    };
    cancelar(id)?;
    *trabalho = None;
    Ok(())
}

#[cfg(test)]
#[path = "backend_testes.rs"]
mod testes_com_a_biblioteca;
