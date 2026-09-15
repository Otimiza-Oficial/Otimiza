import path from "node:path";
import type { NextConfig } from "next";

/*
 * O site é publicado no GitHub Pages, em https://otimiza-oficial.github.io/Otimiza/.
 * Isso pede três coisas:
 *
 * - `output: "export"`: o Pages só serve arquivo, não roda Node.
 * - `basePath`: o site mora na subpasta /Otimiza. A esteira passa
 *   PAGES_BASE_PATH=/Otimiza; no computador fica vazio e tudo abre na raiz.
 * - `images.unoptimized`: o otimizador de imagens do Next precisa de servidor.
 *   As capturas já são PNG leves e os retratos vêm recortados pelo Unsplash.
 */
const basePath = process.env.PAGES_BASE_PATH ?? "";

const nextConfig: NextConfig = {
  output: "export",
  trailingSlash: true,
  basePath,
  images: { unoptimized: true },
  // Links do `next/link` recebem o basePath sozinhos; `src` de imagem e
  // `url()` de CSS não. Os componentes leem daqui para prefixar.
  env: { NEXT_PUBLIC_BASE_PATH: basePath },
  turbopack: { root: path.join(__dirname) },
};

export default nextConfig;
