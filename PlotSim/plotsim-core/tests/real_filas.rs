//! Testes contra o Windows de verdade. Mudam o sistema, então só rodam à mão, com `--ignored`.
//! Criam só filas com o prefixo do simulador e removem o que criaram. Enviam só a essas filas.
//! Precisam de `PLOTS_PDFIUM_DIR` e de `PLOTSIM_PLOTS_EXE`, o executável do `plots`.
#![cfg(windows)]

mod apoio;

use apoio::{pastas, pdf_com_formatos, pdf_numerado, saidas, EnvioPeloPlots, A4, A5, CARTA};
use plotsim_core::filas::Filas;
use plotsim_core::leitor::{inspecionar, ler_paginas, ErroLeitura};
use plotsim_core::powershell::{LeitorDoSpooler, PowerShell, ProcessoDoShell};
use plotsim_core::recepcao::{Evento, Observador, Recepcao, TrabalhoRecebido};
use plotsim_core::spooler::{Spooler, SpoolerDoWindows};
use plotsim_core::{FaceDeSaida, ModoFisico, NomeVirtual, Saida, SistemaDeFilas, Situacao};
use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const MODO: ModoFisico = ModoFisico {
    saida: Saida::Traseira,
    face: FaceDeSaida::ParaCima,
};
const PRAZO_DO_ENVIO: Duration = Duration::from_secs(90);
/// Nenhuma outra abertura: quem tenta escrever recebe recusa do sistema.
const SEM_COMPARTILHAR: u32 = 0;
const SO_LEITURA_COMPARTILHADA: u32 = 1;

/// Remove a fila ao sair, mesmo com o teste falhando.
struct FilaTemporaria {
    curto: &'static str,
    filas: Filas<PowerShell<ProcessoDoShell, LeitorDoSpooler>>,
    porta: String,
    pasta: PathBuf,
}

impl FilaTemporaria {
    fn nova(curto: &'static str) -> Self {
        let filas = Filas::novo(PowerShell::do_sistema(), pastas());
        let porta = filas.criar(curto, MODO).unwrap().porta;
        let pasta = saidas().join(NomeVirtual::novo(curto).unwrap().pasta());
        // Nada de um teste anterior pode passar por recebido neste.
        let _ = fs::remove_dir_all(&pasta);
        let _ = fs::remove_dir_all(filas.pastas().historico(&NomeVirtual::novo(curto).unwrap()));
        fs::create_dir_all(&pasta).unwrap();
        Self {
            curto,
            filas,
            porta,
            pasta,
        }
    }

    fn nome_da_fila(&self) -> String {
        NomeVirtual::novo(self.curto).unwrap().fila()
    }

    fn documento(&self, bytes: Vec<u8>) -> PathBuf {
        let caminho = self.pasta.join("documento.pdf");
        fs::write(&caminho, bytes).unwrap();
        caminho
    }

    fn observar(&self) -> (Observador, Receiver<Evento>) {
        let nome = NomeVirtual::novo(self.curto).unwrap();
        let recepcao = Recepcao::nova(
            SpoolerDoWindows,
            nome.fila(),
            PathBuf::from(&self.porta),
            self.filas.pastas().historico(&nome),
        )
        .unwrap();
        let (destino, eventos) = mpsc::channel();
        (Observador::iniciar(recepcao, destino).unwrap(), eventos)
    }
}

impl Drop for FilaTemporaria {
    fn drop(&mut self) {
        println!(
            "remover {}: {:?}",
            self.curto,
            self.filas.remover(self.curto)
        );
    }
}

fn proximo_recebido(eventos: &Receiver<Evento>, prazo: Duration) -> TrabalhoRecebido {
    match eventos.recv_timeout(prazo) {
        Ok(Evento::Recebido(recebido)) => recebido,
        outro => panic!("esperava um trabalho recebido, veio {outro:?}"),
    }
}

/// Escreve no log o que chegou em um arquivo.
fn mostrar_paginas(rotulo: &str, caminho: &Path) {
    println!(
        "{rotulo}: {} bytes; {:?}; leitura com texto: {:?}",
        fs::metadata(caminho).map(|dados| dados.len()).unwrap_or(0),
        inspecionar(caminho),
        ler_paginas(caminho).map(|paginas| paginas.len())
    );
}

#[test]
#[ignore = "muda as filas do Windows"]
fn criar_listar_e_remover_de_verdade() {
    let sistema = PowerShell::do_sistema();
    let antes = sistema.filas().unwrap();
    println!("filas antes: {antes:?}");
    let filas = Filas::novo(sistema, pastas());
    let criada = filas.criar("Teste de filas", MODO).unwrap();
    println!("criada: {criada:?}");
    assert_eq!(criada.situacao, Situacao::Pronta);
    assert!(Path::new(&criada.porta).parent().unwrap().is_dir());
    assert_eq!(filas.criar("Teste de filas", MODO).unwrap(), criada);
    assert_eq!(filas.listar().unwrap(), vec![criada.clone()]);

    let visao = PowerShell::do_sistema().filas().unwrap();
    let nova = visao
        .iter()
        .find(|fila| fila.nome == "PlotSim — Teste de filas")
        .expect("o nome com travessão chega inteiro ao Windows");
    assert_eq!(nova.porta, criada.porta);
    assert_eq!(nova.driver, "Microsoft Print To PDF");
    assert!(PowerShell::do_sistema()
        .porta_existe(&criada.porta)
        .unwrap());

    let removida = filas.remover("Teste de filas").unwrap();
    assert!(removida.fila_removida && removida.porta_removida);
    let repetida = filas.remover("Teste de filas").unwrap();
    assert!(!repetida.fila_removida && !repetida.porta_removida);
    assert!(filas.listar().unwrap().is_empty());
    assert!(!Path::new(&criada.porta).parent().unwrap().exists());
    assert_eq!(PowerShell::do_sistema().filas().unwrap(), antes);
}

/// A pergunta central: a porta de arquivo recebe trabalhos seguidos sem abrir janela, cada um com
/// o arquivo dele? O próximo envio só é respondido depois de o anterior chegar. Os formatos
/// diferentes mostram que cada arquivo é mesmo o do seu envio.
#[test]
#[ignore = "muda as filas do Windows e envia à fila virtual"]
fn envios_seguidos_sem_dialogo_cada_um_com_o_seu_arquivo() {
    let fila = FilaTemporaria::nova("Viabilidade");
    let documento = fila.documento(pdf_com_formatos(&[A4, A4, A5, A5, CARTA, CARTA]));
    let (observador, eventos) = fila.observar();
    let mut envio = EnvioPeloPlots::iniciar(&fila.nome_da_fila(), &documento, 1, &fila.pasta);
    let esperados = [A4, A4, A5, A5, CARTA, CARTA];
    let mut recebidos: Vec<TrabalhoRecebido> = Vec::new();
    for (indice, formato) in esperados.iter().enumerate() {
        assert!(envio.esperar_pergunta(PRAZO_DO_ENVIO), "o envio parou");
        let inicio = Instant::now();
        envio.responder("s");
        let recebido = proximo_recebido(&eventos, PRAZO_DO_ENVIO);
        println!(
            "envio {}: {:?} chegou em {} ms, documento {:?}",
            indice + 1,
            recebido.id,
            inicio.elapsed().as_millis(),
            recebido.documento
        );
        mostrar_paginas("  recebido", &recebido.arquivo);
        let paginas = inspecionar(&recebido.arquivo).unwrap();
        assert_eq!(paginas.len(), 1);
        assert!(
            (paginas[0].largura_mm - apoio::mm(formato.0)).abs() < 1.5
                && (paginas[0].altura_mm - apoio::mm(formato.1)).abs() < 1.5,
            "envio {} chegou com {paginas:?}",
            indice + 1
        );
        // Depois do trabalho, o sistema solta o arquivo: dá para abri-lo sozinho.
        let solto = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(SEM_COMPARTILHAR)
            .open(&fila.porta);
        println!(
            "  arquivo da porta livre depois do envio: {}",
            solto.is_ok()
        );
        recebidos.push(recebido);
    }
    println!("{}", envio.terminar(Duration::from_secs(60)));
    observador.parar();
    let mut arquivos: Vec<_> = recebidos
        .iter()
        .map(|r| fs::read(&r.arquivo).unwrap())
        .collect();
    arquivos.dedup();
    assert_eq!(arquivos.len(), esperados.len());
}

/// O `plots` só pode oferecer o envio seguinte depois que a fila soltou o trabalho e o arquivo está
/// inteiro. Com o driver de PDF, o bit de retido liga antes de a saída ser gravada.
#[test]
#[ignore = "muda as filas do Windows e envia à fila virtual"]
fn plots_so_pede_o_proximo_envio_com_o_arquivo_completo() {
    let fila = FilaTemporaria::nova("Arquivo completo");
    let documento = fila.documento(pdf_numerado(6));
    let nome_fila = fila.nome_da_fila();
    let mut envio = EnvioPeloPlots::iniciar(&nome_fila, &documento, 1, &fila.pasta);
    let mut envios = 0;
    let mut problemas = Vec::new();
    while envio.esperar_pergunta(PRAZO_DO_ENVIO) {
        if envios > 0 {
            let na_fila = SpoolerDoWindows.trabalhos(&nome_fila).unwrap();
            let bytes = fs::read(&fila.porta).unwrap_or_default();
            let inteiro = bytes.windows(5).rev().take(1024).any(|j| j == b"%%EOF");
            println!(
                "pergunta depois do envio {envios}: arquivo com {} bytes, inteiro={inteiro}, fila={na_fila:?}",
                bytes.len()
            );
            if !inteiro {
                problemas.push(envios);
            }
        }
        envios += 1;
        envio.responder("s");
    }
    println!("{}", envio.terminar(Duration::from_secs(60)));
    assert_eq!(envios, 6);
    assert!(
        problemas.is_empty(),
        "perguntou com o arquivo incompleto depois dos envios {problemas:?}"
    );
}

/// Sem esperar nada entre os envios: o `plots` manda o próximo assim que a fila deixa.
#[test]
#[ignore = "muda as filas do Windows e envia à fila virtual"]
fn envios_em_rajada_nao_perdem_trabalho() {
    let fila = FilaTemporaria::nova("Rajada");
    let documento = fila.documento(pdf_numerado(8));
    let (observador, eventos) = fila.observar();
    let mut envio = EnvioPeloPlots::iniciar(&fila.nome_da_fila(), &documento, 1, &fila.pasta);
    while envio.esperar_pergunta(PRAZO_DO_ENVIO) {
        envio.responder("s");
    }
    println!("{}", envio.terminar(Duration::from_secs(60)));
    let mut todos = Vec::new();
    let inicio = Instant::now();
    while todos.len() < 8 && inicio.elapsed() < PRAZO_DO_ENVIO {
        if let Ok(evento) = eventos.recv_timeout(Duration::from_secs(1)) {
            println!("{evento:?}");
            todos.push(evento);
        }
    }
    observador.parar();
    let recebidos = todos
        .iter()
        .filter(|evento| matches!(evento, Evento::Recebido(_)))
        .count();
    assert_eq!(recebidos, 8, "{todos:?}");
}

/// Três programas na mesma fila, para os trabalhos ficarem mesmo um atrás do outro. Todo trabalho
/// tem que aparecer em um evento; o log mostra quantos chegaram com arquivo.
#[test]
#[ignore = "muda as filas do Windows e envia à fila virtual"]
fn tres_programas_ao_mesmo_tempo_na_mesma_fila() {
    let fila = FilaTemporaria::nova("Tres programas");
    let documento = fila.documento(pdf_numerado(2));
    let nome_fila = fila.nome_da_fila();
    let (observador, eventos) = fila.observar();
    let mut envios = [
        EnvioPeloPlots::iniciar(&nome_fila, &documento, 1, &fila.pasta),
        EnvioPeloPlots::iniciar(&nome_fila, &documento, 1, &fila.pasta),
        EnvioPeloPlots::iniciar(&nome_fila, &documento, 1, &fila.pasta),
    ];
    for envio in &mut envios {
        assert!(envio.esperar_pergunta(PRAZO_DO_ENVIO));
    }
    for envio in &mut envios {
        envio.responder("s");
    }
    let mut todos = Vec::new();
    let inicio = Instant::now();
    while todos.len() < 3 && inicio.elapsed() < PRAZO_DO_ENVIO {
        if let Ok(evento) = eventos.recv_timeout(Duration::from_secs(1)) {
            println!("{evento:?}");
            todos.push(evento);
        }
    }
    for envio in envios {
        let _ = envio.terminar(Duration::from_secs(60));
    }
    observador.parar();
    let com_arquivo = todos
        .iter()
        .filter(|evento| matches!(evento, Evento::Recebido(_)))
        .count();
    println!("{com_arquivo} de 3 chegaram com arquivo");
    assert_eq!(todos.len(), 3, "{todos:?}");
}

/// Linha do tempo de um envio: o que o spooler diz, o tamanho do arquivo e se ele está preso.
#[test]
#[ignore = "muda as filas do Windows e envia à fila virtual"]
fn linha_do_tempo_de_um_envio() {
    let fila = FilaTemporaria::nova("Linha do tempo");
    let documento = fila.documento(pdf_numerado(2));
    let nome_fila = fila.nome_da_fila();
    let mut envio = EnvioPeloPlots::iniciar(&nome_fila, &documento, 1, &fila.pasta);
    assert!(envio.esperar_pergunta(PRAZO_DO_ENVIO));
    envio.responder("s");
    let inicio = Instant::now();
    let mut ultimo = String::new();
    while inicio.elapsed() < Duration::from_secs(20) {
        let trabalhos = SpoolerDoWindows.trabalhos(&nome_fila);
        let tamanho = fs::metadata(&fila.porta).map(|dados| dados.len()).ok();
        let preso = tamanho.is_some()
            && OpenOptions::new()
                .read(true)
                .share_mode(SO_LEITURA_COMPARTILHADA)
                .open(&fila.porta)
                .is_err();
        let agora = format!("{trabalhos:?} arquivo={tamanho:?} preso={preso}");
        if agora != ultimo {
            println!("{:>7} ms {agora}", inicio.elapsed().as_millis());
            ultimo = agora;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    mostrar_paginas("no fim", Path::new(&fila.porta));
    println!("{}", envio.terminar(Duration::from_secs(30)));
}

/// O que o spooler faz quando alguém está com o arquivo da porta aberto sem compartilhar.
#[test]
#[ignore = "muda as filas do Windows e envia à fila virtual"]
fn arquivo_da_porta_preso_por_outro_programa() {
    let fila = FilaTemporaria::nova("Arquivo preso");
    let documento = fila.documento(pdf_numerado(2));
    let nome_fila = fila.nome_da_fila();
    fs::write(&fila.porta, b"arquivo que ja existia").unwrap();
    let preso = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(SEM_COMPARTILHAR)
        .open(&fila.porta)
        .unwrap();
    let mut envio = EnvioPeloPlots::iniciar(&nome_fila, &documento, 1, &fila.pasta);
    assert!(envio.esperar_pergunta(PRAZO_DO_ENVIO));
    envio.responder("s");
    let acompanhar = |rotulo: &str, duracao: Duration| {
        let inicio = Instant::now();
        let mut ultimo = String::new();
        while inicio.elapsed() < duracao {
            let trabalhos = SpoolerDoWindows.trabalhos(&nome_fila);
            let tamanho = fs::metadata(&fila.porta).map(|dados| dados.len()).ok();
            let agora = format!("{trabalhos:?} arquivo={tamanho:?}");
            if agora != ultimo {
                println!("{rotulo} {:>6} ms {agora}", inicio.elapsed().as_millis());
                ultimo = agora;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    };
    acompanhar("preso", Duration::from_secs(25));
    drop(preso);
    println!("arquivo solto");
    acompanhar("solto", Duration::from_secs(25));
    mostrar_paginas("no fim", Path::new(&fila.porta));
    println!("{}", envio.terminar(Duration::from_secs(30)));
}

/// De ponta a ponta: PDF com texto, `plots imprimir`, fila virtual, histórico e leitura de volta.
/// O envio do PlotS desenha cada página como imagem, então o texto não volta e a leitura precisa
/// dizer isso com clareza, com o número de páginas e o tamanho ainda disponíveis.
#[test]
#[ignore = "muda as filas do Windows e envia à fila virtual"]
fn ponta_a_ponta_le_de_volta_o_que_o_plots_enviou() {
    let fila = FilaTemporaria::nova("Ponta a ponta");
    let documento = fila.documento(pdf_numerado(4));
    let (observador, eventos) = fila.observar();
    let mut envio = EnvioPeloPlots::iniciar(&fila.nome_da_fila(), &documento, 2, &fila.pasta);
    let mut recebidos = Vec::new();
    while envio.esperar_pergunta(PRAZO_DO_ENVIO) {
        envio.responder("s");
        recebidos.push(proximo_recebido(&eventos, PRAZO_DO_ENVIO));
    }
    println!("{}", envio.terminar(Duration::from_secs(60)));
    observador.parar();
    assert_eq!(recebidos.len(), 2);
    for recebido in &recebidos {
        mostrar_paginas(&recebido.documento, &recebido.arquivo);
        let paginas = inspecionar(&recebido.arquivo).unwrap();
        assert_eq!(paginas.len(), 2);
        for pagina in &paginas {
            assert!((pagina.largura_mm - 210.0).abs() < 1.5);
            assert!((pagina.altura_mm - 297.0).abs() < 1.5);
            assert_eq!(pagina.rotacao, 0);
        }
        // Guarda a saída fora da pasta de teste, para olhar à mão.
        let copia = saidas().join(format!("ponta-a-ponta-{:?}.pdf", recebido.id));
        fs::copy(&recebido.arquivo, copia).unwrap();
        assert!(matches!(
            ler_paginas(&recebido.arquivo),
            Err(ErroLeitura::PaginaSemTexto { pagina: 1 })
        ));
    }
}
