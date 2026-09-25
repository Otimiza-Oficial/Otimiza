/**
 * Caminho de um arquivo de `public/`, já com a subpasta do GitHub Pages.
 *
 * `next/link` aplica o basePath sozinho, mas `src` de imagem e `url()` de CSS
 * não: sem isto, a logo e as imagens funcionam no computador e somem no ar.
 */
export const asset = (caminho: string) => `${process.env.NEXT_PUBLIC_BASE_PATH ?? ""}${caminho}`;
