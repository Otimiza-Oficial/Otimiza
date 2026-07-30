"""
Gera a identidade visual da Otimiza a partir de uma única definição matemática.

A marca é um astroide de quatro pontas — |x|^(2/3) + |y|^(2/3) = 1 — com um
crescente removido no canto superior esquerdo, obtido subtraindo uma cópia do
mesmo astroide deslocada na diagonal.

Definir a forma por equação, e não por um arquivo desenhado à mão, garante que
o SVG da interface e todos os PNGs de ícone sejam exatamente a mesma curva em
qualquer resolução.

Uso: python brand/make_logo.py
"""

from PIL import Image
import os

# Cores da marca, lidas da logo original.
INK = (245, 243, 240)   # branco quente da estrela
VOID = (11, 11, 11)     # preto do fundo

# Geometria, em fração do lado da imagem.
RADIUS = 0.46           # meia-largura da estrela
CUT_RADIUS = 0.405      # raio da curva que separa a folha
CUT_GAP = 0.016         # espessura do traço vazado
CUT_OFFSET = 0.062      # deslocamento diagonal da curva, para baixo e à direita
# Expoente menor que o do astroide clássico (2/3): deixa os lados mais côncavos
# e as pontas mais afiadas, como na marca original.
EXPONENT = 0.5

SUPERSAMPLE = 4         # antisserrilhado por amostragem


def inside_astroid(x, y, radius):
    """Ponto dentro do astroide de raio `radius`, centrado na origem."""
    if radius <= 0:
        return False
    return (abs(x) / radius) ** EXPONENT + (abs(y) / radius) ** EXPONENT <= 1.0


def is_ink(x, y):
    """Cor final de um ponto em coordenadas normalizadas (-0.5 a 0.5)."""
    if not inside_astroid(x, y, RADIUS):
        return False

    # O corte é um traço vazado, não um pedaço removido: fica entre dois
    # astroides concêntricos e deslocados, separando a folha do corpo.
    cut_x = x - CUT_OFFSET
    cut_y = y - CUT_OFFSET
    on_cut = inside_astroid(cut_x, cut_y, CUT_RADIUS) and not inside_astroid(
        cut_x, cut_y, CUT_RADIUS - CUT_GAP
    )

    return not on_cut


def render(size, transparent=True):
    """Rasteriza a marca com antisserrilhado por supersampling."""
    big = size * SUPERSAMPLE
    mask = Image.new("L", (big, big), 0)
    pixels = mask.load()

    for row in range(big):
        y = (row + 0.5) / big - 0.5
        for column in range(big):
            x = (column + 0.5) / big - 0.5
            if is_ink(x, y):
                pixels[column, row] = 255

    mask = mask.resize((size, size), Image.LANCZOS)

    background = (0, 0, 0, 0) if transparent else VOID + (255,)
    image = Image.new("RGBA", (size, size), background)
    image.paste(INK + (255,), (0, 0), mask)
    return image


def astroid_path(radius, offset=0.0, steps=240):
    """
    Contorno do astroide como caminho SVG.

    Parametrização exata da superelipse |x/r|^n + |y/r|^n = 1:
        x = r · sinal(cos t) · |cos t|^(2/n)
    Amostrar a paramétrica é mais fiel que aproximar por curvas de Bézier e
    mantém as pontas afiadas. O expoente aqui e o do rasterizador são o mesmo
    número, então vetor e bitmap são a mesma curva.
    """
    import math

    power = 2.0 / EXPONENT
    points = []

    for step in range(steps):
        t = 2.0 * math.pi * step / steps
        cos_t, sin_t = math.cos(t), math.sin(t)
        x = radius * math.copysign(abs(cos_t) ** power, cos_t) + offset
        y = radius * math.copysign(abs(sin_t) ** power, sin_t) + offset
        points.append(f"{50 + x * 100:.3f},{50 + y * 100:.3f}")

    return "M" + "L".join(points) + "Z"


def write_svg(path):
    """
    SVG com regra de preenchimento `evenodd`: o segundo contorno vira o recorte
    do crescente sem precisar de máscara nem de operação booleana.
    """
    outer = astroid_path(RADIUS)
    cut_outer = astroid_path(CUT_RADIUS, CUT_OFFSET)
    cut_inner = astroid_path(CUT_RADIUS - CUT_GAP, CUT_OFFSET)

    # `evenodd` com três contornos: o corpo, o vazado do traço e o miolo que
    # volta a ser tinta. Uma única forma, sem máscara.
    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" role="img" aria-label="Otimiza">
  <path fill="currentColor" fill-rule="evenodd" d="{outer} {cut_outer} {cut_inner}"/>
</svg>
'''
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(svg)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    icons = os.path.abspath(os.path.join(here, "..", "src-tauri", "icons"))

    write_svg(os.path.join(here, "logo.svg"))
    print("logo.svg")

    # A interface consome o SVG como máscara CSS, o que permite pintar a marca
    # com a cor do tema sem duplicar o arquivo para cada cor.
    public = os.path.abspath(os.path.join(here, "..", "public"))
    os.makedirs(public, exist_ok=True)
    write_svg(os.path.join(public, "logo.svg"))
    print("public/logo.svg")

    # Ícones do aplicativo. Os quadrados nomeados são exigidos pelo empacotador
    # da Microsoft Store; os demais alimentam a barra de tarefas e a janela.
    sizes = {
        "32x32.png": 32,
        "128x128.png": 128,
        "128x128@2x.png": 256,
        "icon.png": 512,
        "StoreLogo.png": 50,
        "Square30x30Logo.png": 30,
        "Square44x44Logo.png": 44,
        "Square71x71Logo.png": 71,
        "Square89x89Logo.png": 89,
        "Square107x107Logo.png": 107,
        "Square142x142Logo.png": 142,
        "Square150x150Logo.png": 150,
        "Square284x284Logo.png": 284,
        "Square310x310Logo.png": 310,
    }

    cache = {}
    for name, size in sorted(sizes.items(), key=lambda item: -item[1]):
        if size not in cache:
            cache[size] = render(size)
        cache[size].save(os.path.join(icons, name))
        print(name)

    # O .ico carrega várias resoluções: o Windows escolhe conforme o contexto
    # (barra de tarefas, alt-tab, propriedades do arquivo).
    ico_sizes = [16, 24, 32, 48, 64, 128, 256]
    for size in ico_sizes:
        if size not in cache:
            cache[size] = render(size)

    cache[256].save(
        os.path.join(icons, "icon.ico"),
        format="ICO",
        sizes=[(size, size) for size in ico_sizes],
    )
    print("icon.ico")


if __name__ == "__main__":
    main()
