This is a [Next.js](https://nextjs.org) project bootstrapped with [`create-next-app`](https://nextjs.org/docs/app/api-reference/cli/create-next-app).

## Getting Started

First, run the development server:

```bash
npm run dev
# or
yarn dev
# or
pnpm dev
# or
bun dev
```

Open [http://localhost:3000](http://localhost:3000) with your browser to see the result.

You can start editing the page by modifying `app/page.tsx`. The page auto-updates as you edit the file.

This project uses [`next/font`](https://nextjs.org/docs/app/building-your-application/optimizing/fonts) to automatically optimize and load [Geist](https://vercel.com/font), a new font family for Vercel.

## Learn More

To learn more about Next.js, take a look at the following resources:

- [Next.js Documentation](https://nextjs.org/docs) - learn about Next.js features and API.
- [Learn Next.js](https://nextjs.org/learn) - an interactive Next.js tutorial.

You can check out [the Next.js GitHub repository](https://github.com/vercel/next.js) - your feedback and contributions are welcome!

## Deploy on Vercel

The easiest way to deploy your Next.js app is to use the [Vercel Platform](https://vercel.com/new?utm_medium=default-template&filter=next.js&utm_source=create-next-app&utm_campaign=create-next-app-readme) from the creators of Next.js.

Check out our [Next.js deployment documentation](https://nextjs.org/docs/app/building-your-application/deploying) for more details.

## A política de conteúdo (CSP)

O site é estático no GitHub Pages, e o Pages não deixa configurar cabeçalho.
A política vai numa `<meta>` carimbada em cada página depois do build, por
`scripts/csp.mjs` (roda sozinho no `postbuild`).

- **`script-src` sem `'unsafe-inline'`**: os scripts em linha que o Next gera
  entram por hash SHA-256, calculado página a página. Script injetado não roda
  — conferido no navegador, com a violação aparecendo no console.
- **`style-src` com `'unsafe-inline'`**, de propósito: React e Framer Motion
  escrevem `style="..."` no elemento, que é como animação funciona. Sem
  servidor não há nonce.
- **`connect-src`** só permite a API pública do GitHub, que é a única chamada
  que o site faz.
- **`frame-ancestors` e `report-uri` não funcionam por `<meta>`.** Enquanto o
  site for servido pelo Pages, não há como proibir enquadramento por aqui; isso
  exige domínio próprio com cabeçalho na frente.

A esteira reprova a publicação se alguma página sair sem a política, ou se o
`script-src` voltar a liberar `'unsafe-inline'`.
