<script lang="ts">
  // O selo de estado (1.0.12, Frente 1.1): ponto + texto curto. A cor está só no ponto (a marca é monocromática; cor
  // existe para estado), e o texto diz o estado em palavras, para quem não distingue cor e para o Alto Contraste.
  let {
    tom = "neutro",
    texto,
    forte = false,
  }: {
    tom?: "ok" | "atencao" | "perigo" | "neutro";
    texto: string;
    /** Em pílula, para o estado principal de um cartão. */
    forte?: boolean;
  } = $props();
</script>

<span class="selo-de-estado" class:forte data-tom={tom}>
  <span class="ponto" data-tom={tom === "neutro" ? undefined : tom} aria-hidden="true"></span>
  {texto}
</span>

<style>
  .selo-de-estado {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--tinta-2);
    font-size: var(--t-apoio);
    font-weight: 500;
    line-height: 1.4;
    white-space: nowrap;
  }
  .selo-de-estado.forte {
    padding: 3px 10px 3px 8px;
    border: 1px solid var(--linha);
    border-radius: var(--raio-pilula);
    background: var(--superficie-2);
    color: var(--tinta);
  }
  .ponto {
    width: 7px;
    height: 7px;
  }
</style>
