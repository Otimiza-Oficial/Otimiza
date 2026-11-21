import type { MetadataRoute } from "next";
import { asset } from "@/lib/asset";
import { site } from "@/lib/site";

// O `output: "export"` só gera rota de metadados estática.
export const dynamic = "force-static";

/*
 * O manifesto da marca v2 (`otimiza-marca-v2/site/site.webmanifest`). Endereço com `asset`: o site mora em /Otimiza
 * no Pages, e um "/icon-192.png" cru apontaria para a raiz do domínio, onde o arquivo não existe.
 */
export default function manifest(): MetadataRoute.Manifest {
  return {
    name: site.name,
    short_name: site.name,
    start_url: asset("/"),
    scope: asset("/"),
    display: "standalone",
    theme_color: "#000000",
    background_color: "#000000",
    icons: [
      { src: asset("/icon-192.png"), sizes: "192x192", type: "image/png" },
      { src: asset("/icon-512.png"), sizes: "512x512", type: "image/png" },
      { src: asset("/icon-maskable-512.png"), sizes: "512x512", type: "image/png", purpose: "maskable" },
    ],
  };
}
