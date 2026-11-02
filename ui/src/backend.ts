import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { t } from "./i18n";
import type { LoteCaminhos } from "./lote";
import type { Documento } from "./tabs";

export function abrirDocumento(caminho: string): Promise<Documento> {
  return invoke<Documento>("abrir_documento", { caminho });
}

export function caminhosIniciais(): Promise<LoteCaminhos> {
  return invoke<LoteCaminhos>("caminhos_iniciais");
}

/** O limite do lote vale igual para a linha de comando e para o arrasto, e fica no backend. */
export function limitarLote(caminhos: string[]): Promise<LoteCaminhos> {
  return invoke<LoteCaminhos>("limitar_lote", { caminhos });
}

/**
 * Diz ao backend que a interface esta viva e recebeu o pedido de fechamento. Sem este aviso ele
 * destroi a janela ao fim do prazo, entao o aviso vem antes de qualquer dialogo.
 */
export function avisarPedidoDeFechamentoRecebido(): Promise<void> {
  return invoke<void>("pedido_de_fechamento_recebido");
}

export function escolherArquivo(): Promise<string | null> {
  return open({
    multiple: false,
    directory: false,
    filters: [{ name: t("seletor.filtro"), extensions: ["pdf"] }],
  });
}

/**
 * Liga uma assinatura de evento assincrona a um efeito do React e devolve como desfaze-la,
 * mesmo que o desfazer venha antes de a assinatura terminar.
 */
export function assinar(assinatura: Promise<() => void>): () => void {
  let ativa = true;
  let cancelar: (() => void) | null = null;
  assinatura
    .then((funcao) => {
      if (ativa) cancelar = funcao;
      else funcao();
    })
    .catch(() => {
      // Sem o evento so se perde o que depende dele; os outros caminhos da interface seguem.
    });
  return () => {
    ativa = false;
    cancelar?.();
  };
}
