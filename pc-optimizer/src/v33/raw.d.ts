// O Vite entrega `arquivo?raw` como texto: a marca entra no código, sem fetch nem caminho.
declare module "*.svg?raw" {
  const conteudo: string;
  export default conteudo;
}
