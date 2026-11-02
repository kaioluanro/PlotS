export type Idioma = "pt-BR" | "en";

const ptBR = {
  "barra.abas": "Documentos abertos",
  "barra.abrir": "Abrir PDF",
  "barra.lateral.mostrar": "Mostrar barra lateral",
  "barra.lateral.ocultar": "Ocultar barra lateral",
  "barra.minimizar": "Minimizar",
  "barra.maximizar": "Maximizar",
  "barra.restaurar": "Restaurar",
  "barra.fechar": "Fechar janela",
  "aba.fechar": "Fechar {nome}",
  "lateral.titulo": "Impressoras",
  "lateral.total": "{n} na lista",
  "lateral.atualizar": "Atualizar lista",
  "lateral.atualizando": "Lendo as impressoras…",
  "lateral.vazia": "Nenhuma impressora para mostrar. Tente Atualizar lista.",
  "lateral.semDocumento": "Abra um PDF para marcar impressoras.",
  "impressora.simulada.traseira": "Simulada — Traseira",
  "impressora.simulada.gaveta": "Simulada — Gaveta",
  "estado.pronta": "Pronta",
  "estado.desconhecido": "Estado desconhecido",
  "condicao.pausada": "pausada",
  "condicao.offline": "offline",
  "condicao.sem_papel": "sem papel",
  "condicao.atolada": "atolada",
  "condicao.erro": "erro",
  "condicao.exclusao_pendente": "exclusão pendente",
  "condicao.alimentacao_manual": "alimentação manual",
  "condicao.problema_com_papel": "problema com papel",
  "condicao.recebendo_dados": "recebendo dados",
  "condicao.ocupada": "ocupada",
  "condicao.imprimindo": "imprimindo",
  "condicao.bandeja_cheia": "bandeja de saída cheia",
  "condicao.indisponivel": "indisponível",
  "condicao.em_espera": "em espera",
  "condicao.processando": "processando",
  "condicao.inicializando": "inicializando",
  "condicao.aquecendo": "aquecendo",
  "condicao.pouco_toner": "pouco toner",
  "condicao.sem_toner": "sem toner",
  "condicao.pagina_nao_impressa": "página não impressa",
  "condicao.intervencao_do_usuario": "precisa de intervenção",
  "condicao.sem_memoria": "sem memória",
  "condicao.tampa_aberta": "tampa aberta",
  "condicao.servidor_desconhecido": "servidor desconhecido",
  "condicao.economia_de_energia": "economia de energia",
  "condicao.servidor_offline": "servidor offline",
  "condicao.driver_desatualizado": "driver precisa de atualização",
  "condicao.outro": "outro ({detalhe})",
  "modo.traseira": "Traseira ↑",
  "modo.traseira.face": "face para cima",
  "modo.gaveta": "Gaveta ↓",
  "modo.gaveta.face": "face para baixo",
  "modo.calibrar": "Calibrar…",
  "modo.calibrar.titulo": "Esta impressora ainda não tem perfil. Veja como calibrar.",
  "modo.desconhecido": "Perfil não lido",
  "modo.desconhecido.titulo":
    "O arquivo de perfis não pôde ser lido, então o modo desta impressora é desconhecido. Veja o aviso na lista.",
  "modo.perfil.titulo":
    "{modo} {face}. Definido pelo perfil salvo desta impressora; não muda aqui.",
  "modo.simulada.titulo": "{modo} {face}. Modo fixo desta impressora simulada.",
  "aviso.filas":
    "Não foi possível ler as filas do Windows. Confira o serviço de impressão e use Atualizar lista.",
  "aviso.filasCodigo":
    "Não foi possível ler as filas do Windows (código {detalhe}). Confira o serviço de impressão e use Atualizar lista.",
  "aviso.perfis_pasta":
    "O Windows não informou a pasta de configurações deste usuário. O modo das impressoras ficou desconhecido. Entre de novo no Windows e reabra o PlotS.",
  "aviso.perfis_leitura":
    "Não foi possível ler o arquivo de perfis. O modo das impressoras ficou desconhecido. Confira a permissão do arquivo e use Atualizar lista.",
  "aviso.perfis_grande":
    "O arquivo de perfis passa de 1 MB e não foi lido. O modo das impressoras ficou desconhecido. Restaure uma cópia ou remova o arquivo e calibre de novo.",
  "aviso.perfis_invalido":
    "O arquivo de perfis está danificado e não foi lido. O modo das impressoras ficou desconhecido. Restaure uma cópia ou remova o arquivo e calibre de novo.",
  "aviso.perfis_versao":
    "O arquivo de perfis é de uma versão mais nova do PlotS e não foi lido. O modo das impressoras ficou desconhecido. Atualize o PlotS.",
  "aviso.comunicacao": "Não foi possível obter a lista de impressoras. Use Atualizar lista.",
  "folhas.rotulo": "Folhas por lote",
  "folhas.intervalo": "De 1 a 50 folhas em cada lote.",
  "folhas.invalido": "Use de 1 a 50 folhas.",
  "folhas.contando": "Contando páginas…",
  "folhas.recusado": "PDF recusado",
  "folhas.paginas.um": "{n} pág",
  "folhas.paginas.varios": "{n} pág",
  "folhas.lotes.um": "{n} lote",
  "folhas.lotes.varios": "{n} lotes",
  "imprimir.frente": "Imprimir frente",
  "imprimir.frenteContagem": "Imprimir frente ({n})",
  "imprimir.indisponivel": "O envio à impressora ainda não está disponível nesta versão.",
  "calibrar.titulo": "Calibrar {nome}",
  "calibrar.intro": "Esta versão ainda não calibra pela janela. No PowerShell, use:",
  "calibrar.instalacao":
    "O PlotS ainda não instala este comando: rode-o na pasta do executável plots-cli, gerado ao compilar o projeto.",
  "calibrar.passos":
    "O comando grava calibracao.pdf na pasta atual e pergunta o que saiu. O PlotS não envia nada à impressora: imprima as 4 páginas pelo programa que preferir.",
  "calibrar.conferir":
    "O resultado só vale depois de conferido em um lote impresso de verdade. Ao terminar, use Atualizar lista para ver o modo desta impressora.",
  "calibrar.semComando":
    "O nome desta impressora tem aspas duplas e %, e o PlotS não consegue montar um comando seguro para o PowerShell. Renomeie a impressora nas configurações do Windows e use Atualizar lista.",
  "calibrar.copiar": "Copiar comando",
  "calibrar.copiado": "Copiado",
  "calibrar.copiaFalhou": "Não foi possível copiar. Selecione o comando e use Ctrl+C.",
  "calibrar.fechar": "Fechar",
  "documento.vazioTitulo": "Nenhum PDF aberto",
  "documento.vazioTexto": "Abra um PDF ou arraste o arquivo para esta janela.",
  "documento.abrir": "Abrir PDF",
  "documento.atalhoAbrir": "Ctrl+O",
  "documento.semVisualizacao": "Esta versão ainda não mostra o documento.",
  "documento.soltar": "Solte para abrir",
  "seletor.filtro": "Documentos PDF",
  "confirmar.titulo": "Fechar aba com impressão em andamento?",
  "confirmar.texto": "A impressão de {nome} ainda não terminou.",
  "confirmar.consequencia":
    "O que já foi enviado continua na fila de impressão, mas o PlotS deixa de acompanhar este documento e o verso não poderá ser impresso por aqui.",
  "confirmar.consequenciaVarios":
    "O que já foi enviado continua na fila de impressão, mas o PlotS deixa de acompanhar estes documentos e o verso não poderá ser impresso por aqui.",
  "confirmar.manter": "Manter aba aberta",
  "confirmar.fechar": "Fechar aba",
  "confirmar.janela.titulo": "Fechar janela com impressão em andamento?",
  "confirmar.janela.textoVarios": "A impressão de {n} documentos ainda não terminou.",
  "confirmar.janela.manter": "Manter janela aberta",
  "confirmar.janela.fechar": "Fechar janela",
  "erro.nao_encontrado":
    "Não foi possível abrir {nome}: o arquivo não existe. Verifique se ele foi movido ou renomeado.",
  "erro.nao_e_arquivo":
    "Não foi possível abrir {nome}: o caminho não é um arquivo. Escolha um arquivo PDF.",
  "erro.nao_e_pdf":
    "Não foi possível abrir {nome}: não é um PDF. Escolha um arquivo com extensão .pdf.",
  "erro.inacessivel":
    "Não foi possível abrir {nome}: o sistema não permitiu a leitura. Verifique o caminho e a permissão do arquivo.",
  "erro.pdf_invalido":
    "{nome} foi recusado: o PDF está danificado ou usa um recurso que o PlotS não aceita. Salve uma nova cópia pelo programa de origem e abra de novo.",
  "erro.criptografado":
    "{nome} foi recusado: o PDF está protegido por senha. Se você tiver permissão para isso, salve uma cópia sem senha e abra de novo.",
  "erro.grande": "{nome} foi recusado: o arquivo passa de 1 GB ou não coube na memória.",
  "erro.desconhecido": "Não foi possível abrir o documento. Tente de novo.",
  "erro.varios": "{n} arquivos não abriram.",
  "erro.seletor":
    "Não foi possível mostrar o seletor de arquivo. Tente de novo ou arraste o PDF para a janela.",
  "erro.dispensar": "Dispensar aviso",
  "lote.cortado": "Só os primeiros 32 arquivos foram abertos.",
} as const;

export type Chave = keyof typeof ptBR;

const en: Record<Chave, string> = {
  "barra.abas": "Open documents",
  "barra.abrir": "Open PDF",
  "barra.lateral.mostrar": "Show sidebar",
  "barra.lateral.ocultar": "Hide sidebar",
  "barra.minimizar": "Minimize",
  "barra.maximizar": "Maximize",
  "barra.restaurar": "Restore",
  "barra.fechar": "Close window",
  "aba.fechar": "Close {nome}",
  "lateral.titulo": "Printers",
  "lateral.total": "{n} in the list",
  "lateral.atualizar": "Refresh list",
  "lateral.atualizando": "Reading printers…",
  "lateral.vazia": "No printers to show. Try Refresh list.",
  "lateral.semDocumento": "Open a PDF to select printers.",
  "impressora.simulada.traseira": "Simulated — Rear",
  "impressora.simulada.gaveta": "Simulated — Tray",
  "estado.pronta": "Ready",
  "estado.desconhecido": "Unknown state",
  "condicao.pausada": "paused",
  "condicao.offline": "offline",
  "condicao.sem_papel": "out of paper",
  "condicao.atolada": "paper jam",
  "condicao.erro": "error",
  "condicao.exclusao_pendente": "deletion pending",
  "condicao.alimentacao_manual": "manual feed",
  "condicao.problema_com_papel": "paper problem",
  "condicao.recebendo_dados": "receiving data",
  "condicao.ocupada": "busy",
  "condicao.imprimindo": "printing",
  "condicao.bandeja_cheia": "output tray full",
  "condicao.indisponivel": "unavailable",
  "condicao.em_espera": "waiting",
  "condicao.processando": "processing",
  "condicao.inicializando": "initializing",
  "condicao.aquecendo": "warming up",
  "condicao.pouco_toner": "low toner",
  "condicao.sem_toner": "out of toner",
  "condicao.pagina_nao_impressa": "page not printed",
  "condicao.intervencao_do_usuario": "needs attention",
  "condicao.sem_memoria": "out of memory",
  "condicao.tampa_aberta": "door open",
  "condicao.servidor_desconhecido": "unknown server",
  "condicao.economia_de_energia": "power saving",
  "condicao.servidor_offline": "server offline",
  "condicao.driver_desatualizado": "driver needs an update",
  "condicao.outro": "other ({detalhe})",
  "modo.traseira": "Rear ↑",
  "modo.traseira.face": "face up",
  "modo.gaveta": "Tray ↓",
  "modo.gaveta.face": "face down",
  "modo.calibrar": "Calibrate…",
  "modo.calibrar.titulo": "This printer has no profile yet. See how to calibrate it.",
  "modo.desconhecido": "Profile not read",
  "modo.desconhecido.titulo":
    "The profiles file could not be read, so the mode of this printer is unknown. See the notice in the list.",
  "modo.perfil.titulo":
    "{modo} {face}. Set by the saved profile of this printer; it does not change here.",
  "modo.simulada.titulo": "{modo} {face}. Fixed mode of this simulated printer.",
  "aviso.filas":
    "Could not read the Windows print queues. Check the print service and use Refresh list.",
  "aviso.filasCodigo":
    "Could not read the Windows print queues (code {detalhe}). Check the print service and use Refresh list.",
  "aviso.perfis_pasta":
    "Windows did not report this user's settings folder. The mode of the printers is unknown. Sign in to Windows again and open PlotS again.",
  "aviso.perfis_leitura":
    "Could not read the profiles file. The mode of the printers is unknown. Check the file permission and use Refresh list.",
  "aviso.perfis_grande":
    "The profiles file is over 1 MB and was not read. The mode of the printers is unknown. Restore a copy or remove the file and calibrate again.",
  "aviso.perfis_invalido":
    "The profiles file is damaged and was not read. The mode of the printers is unknown. Restore a copy or remove the file and calibrate again.",
  "aviso.perfis_versao":
    "The profiles file comes from a newer version of PlotS and was not read. The mode of the printers is unknown. Update PlotS.",
  "aviso.comunicacao": "Could not get the list of printers. Use Refresh list.",
  "folhas.rotulo": "Sheets per batch",
  "folhas.intervalo": "From 1 to 50 sheets in each batch.",
  "folhas.invalido": "Use 1 to 50 sheets.",
  "folhas.contando": "Counting pages…",
  "folhas.recusado": "PDF refused",
  "folhas.paginas.um": "{n} page",
  "folhas.paginas.varios": "{n} pages",
  "folhas.lotes.um": "{n} batch",
  "folhas.lotes.varios": "{n} batches",
  "imprimir.frente": "Print front",
  "imprimir.frenteContagem": "Print front ({n})",
  "imprimir.indisponivel": "Sending to the printer is not available in this version yet.",
  "calibrar.titulo": "Calibrate {nome}",
  "calibrar.intro": "This version cannot calibrate from the window yet. In PowerShell, use:",
  "calibrar.instalacao":
    "PlotS does not install this command yet: run it in the folder of the plots-cli executable, created when you build the project.",
  "calibrar.passos":
    "The command writes calibracao.pdf in the current folder and asks what came out. PlotS sends nothing to the printer: print the 4 pages from any program you like.",
  "calibrar.conferir":
    "The result only counts after you check it on a batch that was really printed. When done, use Refresh list to see the mode of this printer.",
  "calibrar.semComando":
    "This printer's name has double quotes and %, and PlotS cannot build a safe PowerShell command for it. Rename the printer in Windows settings, then use Refresh list.",
  "calibrar.copiar": "Copy command",
  "calibrar.copiado": "Copied",
  "calibrar.copiaFalhou": "Could not copy. Select the command and press Ctrl+C.",
  "calibrar.fechar": "Close",
  "documento.vazioTitulo": "No PDF open",
  "documento.vazioTexto": "Open a PDF or drag the file into this window.",
  "documento.abrir": "Open PDF",
  "documento.atalhoAbrir": "Ctrl+O",
  "documento.semVisualizacao": "This version does not show the document yet.",
  "documento.soltar": "Drop to open",
  "seletor.filtro": "PDF documents",
  "confirmar.titulo": "Close tab with a print in progress?",
  "confirmar.texto": "{nome} is still printing.",
  "confirmar.consequencia":
    "Anything already sent stays in the print queue, but PlotS stops tracking this document and the back side can no longer be printed from here.",
  "confirmar.consequenciaVarios":
    "Anything already sent stays in the print queue, but PlotS stops tracking these documents and the back side can no longer be printed from here.",
  "confirmar.manter": "Keep tab open",
  "confirmar.fechar": "Close tab",
  "confirmar.janela.titulo": "Close window with a print in progress?",
  "confirmar.janela.textoVarios": "{n} documents are still printing.",
  "confirmar.janela.manter": "Keep window open",
  "confirmar.janela.fechar": "Close window",
  "erro.nao_encontrado":
    "Could not open {nome}: the file does not exist. Check whether it was moved or renamed.",
  "erro.nao_e_arquivo": "Could not open {nome}: the path is not a file. Choose a PDF file.",
  "erro.nao_e_pdf": "Could not open {nome}: it is not a PDF. Choose a file with the .pdf extension.",
  "erro.inacessivel":
    "Could not open {nome}: the system did not allow reading it. Check the path and the file permission.",
  "erro.pdf_invalido":
    "{nome} was refused: the PDF is damaged or uses a feature PlotS does not accept. Save a new copy from the original program and open it again.",
  "erro.criptografado":
    "{nome} was refused: the PDF is password protected. If you are allowed to, save a copy without a password and open it again.",
  "erro.grande": "{nome} was refused: the file is over 1 GB or did not fit in memory.",
  "erro.desconhecido": "Could not open the document. Try again.",
  "erro.varios": "{n} files did not open.",
  "erro.seletor": "Could not show the file picker. Try again or drag the PDF into the window.",
  "erro.dispensar": "Dismiss message",
  "lote.cortado": "Only the first 32 files were opened.",
};

export const traducoes: Record<Idioma, Record<Chave, string>> = { "pt-BR": ptBR, en };

/** Inglês só quando o sistema pede inglês; qualquer outro idioma cai em português. */
export function escolherIdioma(idiomaDoSistema: string | undefined): Idioma {
  return /^en(-|$)/i.test(idiomaDoSistema ?? "") ? "en" : "pt-BR";
}

export function traduzir(
  idioma: Idioma,
  chave: Chave,
  parametros: Record<string, string> = {},
): string {
  return traducoes[idioma][chave].replace(
    /\{(\w+)\}/g,
    (marcador, nome: string) => parametros[nome] ?? marcador,
  );
}

export const idiomaAtual: Idioma = escolherIdioma(
  typeof navigator === "undefined" ? undefined : navigator.language,
);

export function t(chave: Chave, parametros?: Record<string, string>): string {
  return traduzir(idiomaAtual, chave, parametros);
}
