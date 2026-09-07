// @ts-check
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

// O site e estatico de proposito. Nada aqui precisa de servidor: o checkout
// fala com uma API hospedada fora, porque a chave privada da licenca nunca
// pode existir no lado do cliente.
export default defineConfig({
  site: "https://otimiza-oficial.github.io",
  base: "/Otimiza",
  trailingSlash: "ignore",
  build: { inlineStylesheets: "auto" },
  /* O cast existe por um problema de TIPO, nao de execucao: o @tailwindcss/vite
     traz a sua propria copia do vite, e o astro tem outra aninhada. As duas
     declaram `Plugin` de forma incompativel, e o `astro check` reclama. O
     plugin funciona normalmente — e o `npm run build` roda o check antes do
     build justamente para que uma chave faltando numa traducao vire erro, entao
     ele nao pode tropecar neste ruido. Remova o cast quando as duas versoes do
     vite convergirem. */
  vite: { plugins: [/** @type {any} */ (tailwindcss())] },
});
