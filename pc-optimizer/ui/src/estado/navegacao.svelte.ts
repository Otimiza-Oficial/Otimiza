// Onde a pessoa está: a área aberta, a aba do trilho que ela pertence e a última área de cada aba (1.0.12, Frente 1.2:
// saiu do `App.svelte`). As 4 abas no trilho da esquerda são a intenção; as áreas da aba ativa, embaixo, são o lugar
// (`lib/areas.ts`, `grupos`).
import { areas, grupoDe, type Grupo, type IdDaArea, type IdDoGrupo } from "../lib/areas";

/** O "Ver o sistema" do bloco Agora abre o Sistema já na aba Agora; o resto entra pelos Ajustes. */
export type AbaDoSistema = "agora" | "ajustes";

export const nomeDaAba = (id: IdDaArea) => (id === "inicio" ? "Visão geral" : (areas.find((a) => a.id === id)?.nome ?? id));

export class Navegacao {
  aberta = $state<IdDaArea>("inicio");
  abaDoSistema = $state<AbaDoSistema>("ajustes");
  readonly grupoAtivo = $derived(grupoDe(this.aberta));
  readonly area = $derived(areas.find((a) => a.id === this.aberta) ?? areas[0]);
  /** A última área aberta em cada aba: voltar à aba volta para onde a pessoa estava. */
  #ultimaDoGrupo: Partial<Record<IdDoGrupo, IdDaArea>> = {};

  abrir(id: IdDaArea, abaInterna: AbaDoSistema = "ajustes") {
    this.#ultimaDoGrupo[grupoDe(id).id] = id;
    this.abaDoSistema = abaInterna;
    this.aberta = id;
  }

  /** A área onde a pessoa estava naquela aba (a primeira, na primeira vez). */
  areaDoGrupo(g: Grupo): IdDaArea {
    return this.#ultimaDoGrupo[g.id] ?? g.areas[0];
  }
}
