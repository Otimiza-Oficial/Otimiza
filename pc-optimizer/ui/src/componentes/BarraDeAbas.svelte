<script lang="ts">
  // As áreas da aba ativa, embaixo (o lugar), com o marcador que desliza até a área aberta. São abas de verdade
  // (1.0.12, Frente 1.3): `tablist` com setas, Home e End, e cada aba aponta o painel da casca (`#painel-da-area`).
  import Abas from "./base/Abas.svelte";
  import type { Grupo, IdDaArea } from "../lib/areas";
  import { nomeDaAba } from "../estado/navegacao.svelte";

  let { grupo, aberta, onabrir }: { grupo: Grupo; aberta: IdDaArea; onabrir: (id: IdDaArea) => void } = $props();

  const abas = $derived(grupo.areas.map((id) => ({ id, rotulo: nomeDaAba(id) })));
</script>

<Abas {abas} ativa={aberta} rotulo={`Áreas de ${grupo.nome}`} prefixo="areas" idDoPainel="painel-da-area" variante="barra" onmudar={onabrir} />
