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
  "lateral.vazioTitulo": "Ainda não disponível",
  "lateral.vazioTexto": "Esta versão não lista impressoras.",
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
    "O que já foi enviado continua na fila, mas o PlotS deixa de acompanhar este documento e o verso não poderá ser impresso por aqui.",
  "confirmar.consequenciaVarios":
    "O que já foi enviado continua na fila, mas o PlotS deixa de acompanhar estes documentos e o verso não poderá ser impresso por aqui.",
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
  "lateral.vazioTitulo": "Not available yet",
  "lateral.vazioTexto": "This version does not list printers.",
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
    "Anything already sent stays in the queue, but PlotS stops tracking this document and the back side can no longer be printed from here.",
  "confirmar.consequenciaVarios":
    "Anything already sent stays in the queue, but PlotS stops tracking these documents and the back side can no longer be printed from here.",
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
