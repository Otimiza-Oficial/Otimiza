// O GitHub Pages não deixa mandar `frame-ancestors`. Aberto dentro de outro
// site, o nosso some antes de aparecer: não há botão para ser clicado por engano.
if (window.top !== window.self) {
  document.documentElement.style.display = "none";
  try {
    window.top.location.replace(window.self.location.href);
  } catch (e) {}
}
