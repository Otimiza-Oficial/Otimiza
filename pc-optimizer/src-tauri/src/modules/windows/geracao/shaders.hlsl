// ---------------------------------------------------------------------------
// GERADOR DE QUADROS DO OTIMIZA — shaders
//
// Tudo roda na placa de vídeo, em passes de tela cheia:
//
//   1. Luma      — imagem → luminância em 1/4 e depois 1/16 da resolução
//   2. Camera    — um movimento só para a imagem toda, em 1/16 (o giro da câmera)
//      Grossa    — em 1/16, ao redor do parado e do movimento da câmera
//   3. Fina      — refina em 1/4, com os vetores grossos da vizinhança como
//                  candidatos, na grade de 1/8
//   4. Suave     — mediana vetorial 3×3: tira vetor isolado errado
//      (em teste no lugar de Grossa/Fina/Suave: Busca + Amplia, o fluxo por
//      blocos do AMD — ver `busca_por_blocos` em gpu.rs)
//   5. Escolha   — em meia resolução, o vetor em que os dois reais concordam
//                  numa vizinhança de 5 pontos
//   6. Final     — cada pixel do quadro gerado busca a cor seguindo o vetor
//                  nos dois quadros reais; onde os dois discordam (oclusão,
//                  interface, erro de busca) recua para o quadro real mais
//                  próximo em vez de inventar
//
// A busca é SIMÉTRICA: para um ponto p do quadro do meio, compara
// anterior(p − u) com atual(p + u). Assim o vetor já nasce na grade do quadro
// gerado, sem buracos, e o mesmo campo serve para t = 1/3, 1/2, 2/3…
// ---------------------------------------------------------------------------

cbuffer Parametros : register(b0)
{
    float2 texelDestino;   // 1 / tamanho do alvo
    float2 texelOrigem;    // 1 / tamanho da textura 0
    int2   tamanhoOrigem;  // tamanho da textura 0, em pixels
    int2   tamanhoAux;     // tamanho da textura 2, em pixels
    float  t;              // posição do quadro gerado entre os reais
    float  lambda;         // penalidade por vetor longo (prefere parado)
    float  limiarErro;     // a partir de quando A e B "discordam"
    float  faixaErro;
    float  hdr;            // 1 = área de trabalho HDR (FP16 linear): a imagem chega codificada e sai linear
    float  rotacao;        // monitor girado: 0, 1 = 90°, 2 = 180°, 3 = 270° (só no passe Entrada)
    float2 ajusteFluxo;    // Busca: penalidade por pixel longe da previsão, vantagem da mediana dos pais
    float4 extra;          // DisPedaco: x = 1 testa os vizinhos como partida (propagação espacial)
};

Texture2D<float4> T0 : register(t0);
Texture2D<float4> T1 : register(t1);
Texture2D<float4> T2 : register(t2);
Texture2D<float4> T3 : register(t3);
Texture2D<float4> T4 : register(t4);
SamplerState Linear : register(s0);

struct Saida { float4 pos : SV_Position; };

Saida VS(uint id : SV_VertexID)
{
    Saida s;
    float2 uv = float2((id << 1) & 2, id & 2);
    s.pos = float4(uv * float2(2, -2) + float2(-1, 1), 0, 1);
    return s;
}

static const float3 PESOS_LUMA = float3(0.299, 0.587, 0.114);

// HDR. A área de trabalho HDR vem em scRGB linear (1,0 = branco do SDR, o brilho passa disso). Os limiares daqui
// foram calibrados em cor com gama, então a imagem entra codificada com gama 2,2 (Entrada) e só volta a linear
// na saída (Sair). No SDR as duas são identidade.
float3 Sair(float3 c) { return hdr > 0.5 ? pow(max(c, 0.0), 2.2) : c; }

// Diferença entre duas lumas do mesmo tamanho (T0, T1). A média (pelos mips) diz se o quadro capturado é o mesmo
// de antes: num monitor girado, a composição que a própria sobreposição provoca chega como "quadro novo".
float4 Diferenca(Saida e) : SV_Target
{
    int2 p = int2(e.pos.xy);
    return float4(abs(T0.Load(int3(p, 0)).r - T1.Load(int3(p, 0)).r), 0, 0, 1);
}

// Entrada: o recorte da captura (T0, na orientação do PAINEL) vira a imagem na orientação da área de trabalho.
// tamanhoOrigem = tamanho da imagem de saída. Monitor girado: gira de volta; HDR: codifica com gama.
float4 Entrada(Saida e) : SV_Target
{
    int2 p = int2(e.pos.xy);
    int2 t = tamanhoOrigem;
    int2 q = p;
    if (rotacao > 0.5 && rotacao < 1.5) q = int2(p.y, t.x - 1 - p.x);
    else if (rotacao > 1.5 && rotacao < 2.5) q = int2(t.x - 1 - p.x, t.y - 1 - p.y);
    else if (rotacao > 2.5) q = int2(t.y - 1 - p.y, p.x);
    float3 c = T0.Load(int3(q, 0)).rgb;
    return float4(hdr > 0.5 ? pow(max(c, 0.0), 1.0 / 2.2) : c, 1);
}

// Média de um bloco 4×4 com quatro amostras bilineares.
float4 Luma(Saida e) : SV_Target
{
    float2 centro = (floor(e.pos.xy) * 4.0 + 2.0) * texelOrigem;
    float2 o = texelOrigem;
    float4 soma = T0.SampleLevel(Linear, centro + float2(-o.x, -o.y), 0)
                + T0.SampleLevel(Linear, centro + float2( o.x, -o.y), 0)
                + T0.SampleLevel(Linear, centro + float2(-o.x,  o.y), 0)
                + T0.SampleLevel(Linear, centro + float2( o.x,  o.y), 0);
    soma *= 0.25;
    float y = dot(soma.rgb, PESOS_LUMA);
    // A textura de origem já em luma (segundo nível) tem y no canal r.
    return float4(tamanhoAux.x == 1 ? soma.r : y, 0, 0, 1);
}

float Ler0(int2 p) { return T0.Load(int3(clamp(p, int2(0, 0), tamanhoOrigem - 1), 0)).r; }
float Ler1(int2 p) { return T1.Load(int3(clamp(p, int2(0, 0), tamanhoOrigem - 1), 0)).r; }

// Busca global em 1/64. T0/T1 = luma 1/64. Alcança ±384 px de movimento
// simétrico (768 px entre os dois reais): o giro rápido de câmera, que a busca
// em 1/16 sozinha não alcançava — e o quadro gerado saía derretido.
float4 Global(Saida e) : SV_Target
{
    int2 p = int2(e.pos.xy);
    const int R = 6;
    float melhor = 1e9;
    float parado = 1e9;
    int2 escolhido = int2(0, 0);
    [loop] for (int uy = -R; uy <= R; uy++)
    {
        [loop] for (int ux = -R; ux <= R; ux++)
        {
            int2 u = int2(ux, uy);
            float sad = 0;
            [loop] for (int dy = -1; dy <= 1; dy++)
                [loop] for (int dx = -1; dx <= 1; dx++)
                {
                    int2 d = int2(dx, dy);
                    sad += abs(Ler0(p + d - u) - Ler1(p + d + u));
                }
            sad += lambda * 0.5 * (abs(ux) + abs(uy));
            if (sad < melhor) { melhor = sad; escolhido = u; }
            if (ux == 0 && uy == 0) parado = sad;
        }
    }
    if (parado <= melhor * 1.01 + 0.001) escolhido = int2(0, 0);
    // 2u em pixels de 1/64 → ×64 na imagem.
    return float4(float2(escolhido) * 128.0, melhor, 1);
}

// MOVIMENTO DA CÂMERA. A busca global bloco a bloco em 1/64 (3×3 amostras) chutava vetores absurdos (de −384 a
// +128 px num giro de 8 px, medido em quadros do FiveM) e o quadro gerado saía com o cenário em pedaços. Aqui um
// deslocamento só para a imagem inteira: cada pixel do alvo (2R+1)² é um candidato u, e a nota é a diferença média
// sobre o miolo da imagem em 1/16. Robusto porque cada candidato é julgado por milhares de pontos, não por nove.
// T0/T1 = luma 1/16 (tamanhoOrigem).
static const int RAIO_DA_CAMERA = 12;
float4 Camera(Saida e) : SV_Target
{
    int2 u = int2(e.pos.xy) - RAIO_DA_CAMERA;
    int2 lo = tamanhoOrigem / 16;
    int2 hi = tamanhoOrigem - lo;
    float soma = 0;
    float n = 0;
    [loop] for (int y = lo.y; y < hi.y; y += 2)
        [loop] for (int x = lo.x; x < hi.x; x += 2)
        {
            int2 p = int2(x, y);
            // Só o que MUDOU entre os dois reais. Em terceira pessoa a câmera gira em volta do personagem: ele e o HUD
            // ficam parados na tela e o cenário corre atrás. Contando os parados, o centro (o personagem) dizia
            // "a câmera quase não andou" e o cenário nunca achava o movimento dele.
            if (abs(Ler0(p) - Ler1(p)) < 0.03) continue;
            soma += abs(Ler0(p - u) - Ler1(p + u));
            n += 1;
        }
    // Sem nada mudando, todo candidato empata e o parado vence (em CameraMin).
    return float4(n > 0 ? soma / n : 0.0, 0, 0, 1);
}

// O melhor candidato de Camera (T0, (2R+1)²), num pixel só. Parado só ganha empate de verdade.
float4 CameraMin(Saida e) : SV_Target
{
    float melhor = 1e9;
    int2 escolhido = int2(0, 0);
    float parado = T0.Load(int3(RAIO_DA_CAMERA, RAIO_DA_CAMERA, 0)).r;
    [loop] for (int y = 0; y <= 2 * RAIO_DA_CAMERA; y++)
        [loop] for (int x = 0; x <= 2 * RAIO_DA_CAMERA; x++)
        {
            float s = T0.Load(int3(x, y, 0)).r;
            if (s < melhor) { melhor = s; escolhido = int2(x, y) - RAIO_DA_CAMERA; }
        }
    if (parado <= melhor * 1.02) escolhido = int2(0, 0);
    // 2u em pixels de 1/16 → ×16 na imagem.
    return float4(float2(escolhido) * 32.0, melhor, 1);
}

// Busca grossa em 1/16. T0/T1 = luma 1/16; T2 = vetores globais (1/64).
// Procura ao redor de dois pontos de partida: parado e o vetor global.
float4 Grossa(Saida e) : SV_Target
{
    int2 p = int2(e.pos.xy);
    const int R = 4;
    int2 pg = clamp(p / 4, int2(0, 0), tamanhoAux - 1);
    int2 partida[2];
    partida[0] = int2(0, 0);
    partida[1] = int2(round(T2.Load(int3(pg, 0)).xy / 32.0));

    float melhor = 1e9;
    float parado = 1e9;
    int2 escolhido = int2(0, 0);
    [loop] for (int k = 0; k < 2; k++)
    {
        [loop] for (int uy = -R; uy <= R; uy++)
        {
            [loop] for (int ux = -R; ux <= R; ux++)
            {
                int2 u = partida[k] + int2(ux, uy);
                float sad = 0;
                [loop] for (int dy = -2; dy <= 2; dy++)
                    [loop] for (int dx = -2; dx <= 2; dx++)
                    {
                        int2 d = int2(dx, dy);
                        sad += abs(Ler0(p + d - u) - Ler1(p + d + u));
                    }
                sad += lambda * 0.5 * (abs(u.x - partida[k].x) + abs(u.y - partida[k].y));
                if (sad < melhor) { melhor = sad; escolhido = u; }
                if (u.x == 0 && u.y == 0) parado = min(parado, sad);
            }
        }
    }
    // PARADO VENCE EMPATE: textura repetida casa a um período de distância
    // quase tão bem quanto no lugar certo.
    if (parado <= melhor * 1.01 + 0.001) escolhido = int2(0, 0);
    return float4(float2(escolhido) * 32.0, melhor, 1);
}

// Busca fina em 1/4, saída na grade de 1/8. T0/T1 = luma 1/4; T2 = vetores
// grossos (1/16), em pixels da imagem.
//
// ORÇAMENTO. Numa GTX 1650 dividida com o jogo, o passe custava 23 ms em 1080p
// e o gerador perdia metade dos quadros reais. Agora: 4 candidatos (parado,
// grosso do bloco e dois vizinhos) × vizinhança 3×3 × patch 4×4 — e a fração
// de pixel sai de uma parábola sobre as diferenças ao redor do vencedor, em vez
// de um passe inteiro em resolução cheia.
float Sad4(int2 c, int2 u)
{
    float sad = 0;
    [loop] for (int dy = -2; dy <= 1; dy++)
        [loop] for (int dx = -2; dx <= 1; dx++)
        {
            int2 d = int2(dx, dy);
            sad += abs(Ler0(c + d - u) - Ler1(c + d + u));
        }
    return sad;
}

float4 Fina(Saida e) : SV_Target
{
    int2 q = int2(e.pos.xy);
    int2 c = q * 2 + 1;
    int2 qg = clamp(q / 2, int2(0, 0), tamanhoAux - 1);

    int2 predicao[4];
    predicao[0] = int2(0, 0);
    predicao[1] = int2(round(T2.Load(int3(qg, 0)).xy / 8.0));
    predicao[2] = int2(round(T2.Load(int3(clamp(qg + int2((q.x & 1) * 2 - 1, 0), int2(0, 0), tamanhoAux - 1), 0)).xy / 8.0));
    predicao[3] = int2(round(T2.Load(int3(clamp(qg + int2(0, (q.y & 1) * 2 - 1), int2(0, 0), tamanhoAux - 1), 0)).xy / 8.0));

    float melhor = 1e9;
    int2 escolhido = int2(0, 0);
    [loop] for (int k = 0; k < 4; k++)
    {
        [loop] for (int uy = -1; uy <= 1; uy++)
        {
            [loop] for (int ux = -1; ux <= 1; ux++)
            {
                int2 u = predicao[k] + int2(ux, uy);
                float sad = Sad4(c, u) + lambda * 0.5 * (abs(u.x) + abs(u.y));
                if (sad < melhor) { melhor = sad; escolhido = u; }
            }
        }
    }

    // CHÃO LISO NÃO TEM MOVIMENTO PARA ACHAR. Em areia, céu e asfalto todo vetor dá quase a mesma diferença, e o menor
    // dos 36 saía por sorte: o campo ficava cheio de vetores errados (quadradinhos na composição por blocos, ondas na
    // suave). Um vetor só vence o parado se for CLARAMENTE melhor: senão, parado. Onde há textura, o certo ganha com folga.
    float semMover = Sad4(c, int2(0, 0));
    if (melhor > semMover * 0.85 - 0.02) escolhido = int2(0, 0);

    // Fração de pixel: parábola pelas três diferenças em cada eixo.
    float s0 = Sad4(c, escolhido);
    float sxm = Sad4(c, escolhido - int2(1, 0)), sxp = Sad4(c, escolhido + int2(1, 0));
    float sym = Sad4(c, escolhido - int2(0, 1)), syp = Sad4(c, escolhido + int2(0, 1));
    float dx = sxm + sxp - 2.0 * s0;
    float dy = sym + syp - 2.0 * s0;
    float fx = dx > 1e-4 ? clamp(0.5 * (sxm - sxp) / dx, -0.5, 0.5) : 0.0;
    float fy = dy > 1e-4 ? clamp(0.5 * (sym - syp) / dy, -0.5, 0.5) : 0.0;

    // u em pixels de 1/4; vetor total = 2u; na imagem, ×4.
    return float4((float2(escolhido) + float2(fx, fy)) * 8.0, melhor, 1);
}

// Mediana vetorial 3×3. T0 = vetores finos.
float4 Suave(Saida e) : SV_Target
{
    int2 p = int2(e.pos.xy);
    float2 v[9];
    int i = 0;
    for (int dy = -1; dy <= 1; dy++)
        for (int dx = -1; dx <= 1; dx++)
            v[i++] = T0.Load(int3(clamp(p + int2(dx, dy), int2(0, 0), tamanhoOrigem - 1), 0)).xy;

    float melhor = 1e9;
    float2 escolhido = v[4];
    for (int a = 0; a < 9; a++)
    {
        float soma = 0;
        for (int b = 0; b < 9; b++) soma += length(v[a] - v[b]);
        if (soma < melhor) { melhor = soma; escolhido = v[a]; }
    }
    return float4(escolhido, 0, 1);
}

// MÁSCARA DE INTERFACE, em meia resolução. T0 = anterior, T1 = atual,
// T2 = máscara do quadro real anterior. Saída r = fração recente do tempo em
// que este ponto ficou PARADO na tela.
//
// A moldura do velocímetro, as barras de vida, as caixas de notificação e o
// fundo dos textos ficam parados quase sempre — mesmo quando o número dentro
// deles muda. Um ponto parado na maior parte do tempo é tratado como
// interface: sai do quadro real, nunca deslocado nem misturado. É o que acaba
// com os números quebrados do velocímetro.
float4 Mascara(Saida e) : SV_Target
{
    float2 uv = (floor(e.pos.xy) + 0.5) * 2.0 * texelDestino;
    float3 antes = T0.SampleLevel(Linear, uv, 0).rgb;
    float3 depois = T1.SampleLevel(Linear, uv, 0).rgb;
    float parado = dot(abs(antes - depois), float3(0.299, 0.587, 0.114)) < 0.008 ? 1.0 : 0.0;
    float anterior = T2.Load(int3(int2(e.pos.xy), 0)).r;
    return float4(lerp(anterior, parado, 0.08), 0, 0, 1);
}

// Escolha do vetor, em meia resolução. T0 = anterior, T1 = atual (imagem
// inteira), T2 = vetores (1/8). Saída: xy = vetor escolhido, z = discordância.
//
// POR QUE NÃO PIXEL A PIXEL. Decidindo com um pixel só, dois vizinhos
// escolhiam vetores diferentes na borda de quem se move, e cada um trazia uma
// cor de um lugar — os pontinhos soltos na perna e na mão do personagem. Aqui a
// decisão compara cinco pontos em cruz (±2 px), e vizinhos passam a concordar.
static const int2 CRUZ[5] = { int2(0, 0), int2(1, 0), int2(-1, 0), int2(0, 1), int2(0, -1) };
static const float2 TAPS[5] = { float2(0, 0), float2(2, 0), float2(-2, 0), float2(0, 2), float2(0, -2) };

float4 Escolha(Saida e) : SV_Target
{
    float2 pixel = (floor(e.pos.xy) + 0.5) * 2.0;
    float2 uv = pixel * texelDestino;
    int2 bloco = int2(pixel) / 8;

    float melhorErro = 1e9;
    float2 melhorVetor = float2(0, 0);
    [loop] for (int k = 0; k < 6; k++)
    {
        float2 v = float2(0, 0);
        if (k < 5)
            v = T2.Load(int3(clamp(bloco + CRUZ[k], int2(0, 0), tamanhoAux - 1), 0)).xy;
        float2 vuv = v * texelDestino;
        float erro = 0;
        [loop] for (int n = 0; n < 5; n++)
        {
            float2 o = TAPS[n] * texelDestino;
            float3 a = T0.SampleLevel(Linear, uv + o - vuv * t, 0).rgb;
            float3 b = T1.SampleLevel(Linear, uv + o + vuv * (1.0 - t), 0).rgb;
            erro += dot(abs(a - b), float3(0.299, 0.587, 0.114));
        }
        erro *= 0.2;
        // O vetor do próprio bloco ganha empate: estabilidade entre quadros.
        if (k == 0) erro -= 0.004;
        if (erro < melhorErro) { melhorErro = erro; melhorVetor = v; }
    }
    // Canal a: 1 onde a discordância é grande. A média disso na tela inteira
    // é a nota do quadro — e decide se ele pode ser mostrado.
    melhorErro = max(melhorErro, 0.0);
    // Segundo sinal: quanto este ponto MUDOU de verdade entre os dois reais.
    // Câmera atravessando o interior de um carro muda a tela inteira, e em
    // superfícies lisas a cor ainda "bate" com vetor errado. Mudança grande
    // com concordância só razoável também conta como ruim.
    float3 antes = T0.SampleLevel(Linear, uv, 0).rgb;
    float3 depois = T1.SampleLevel(Linear, uv, 0).rgb;
    float mudou = dot(abs(antes - depois), float3(0.299, 0.587, 0.114));
    bool ruim = melhorErro > 0.05 || (mudou > 0.18 && melhorErro > 0.02);
    return float4(melhorVetor, melhorErro, ruim ? 1.0 : 0.0);
}

// Composição em resolução cheia. T0 = anterior, T1 = atual, T2 = escolha (1/2).
//
// Onde os dois quadros concordam: mistura os dois deslocados. Onde não
// concordam — quase sempre fundo que acabou de aparecer atrás de quem se
// move —, a cor vem de UM lado só, deslocado pelo mesmo vetor: o quadro mais
// próximo no tempo. Misturar os dois no lugar, como antes, deixava fantasma.
float4 Final(Saida e) : SV_Target
{
    // Diagnóstico (OTIMIZA_FG_VER_VETORES): o vetor escolhido em cor, cinza = parado.
    if (ajusteFluxo.x < -0.5)
        return float4(saturate(0.5 + T2.Load(int3(int2(e.pos.xy) / 2, 0)).xy / 64.0), 0.5, 1);
    float2 uv = e.pos.xy * texelDestino;
    float3 anteriorNoLugar = T0.SampleLevel(Linear, uv, 0).rgb;
    float3 atualNoLugar = T1.SampleLevel(Linear, uv, 0).rgb;

    // Interface (ver Mascara): sai do real mais próximo, no lugar.
    if (T3.Load(int3(int2(e.pos.xy) / 2, 0)).r > 0.5)
        return float4(Sair(t < 0.5 ? anteriorNoLugar : atualNoLugar), 1);

    // PIXEL PARADO SAI DO REAL. Se o pixel não mudou entre os dois quadros
    // reais — HUD, minimapa, velocímetro, texto — ele não é deslocado por
    // vetor nenhum. É o que mantém a interface nítida com a câmera girando.
    if (dot(abs(anteriorNoLugar - atualNoLugar), float3(0.3333, 0.3333, 0.3333)) < 0.004)
        return float4(Sair(atualNoLugar), 1);

    // BORDA SEM DEGRAU. A escolha vem em meia resolução; perto da borda de um
    // objeto, o pixel testa a escolha do seu ponto e das quatro vizinhas e fica
    // com a que faz os dois quadros concordarem NELE.
    int2 m = int2(e.pos.xy) / 2;
    int2 tamanhoEscolha = int2(ceil(1.0 / texelDestino * 0.5));
    static const int2 VIZ[5] = { int2(0, 0), int2(1, 0), int2(-1, 0), int2(0, 1), int2(0, -1) };
    float4 escolha = T2.Load(int3(m, 0));
    float3 a = T0.SampleLevel(Linear, uv - escolha.xy * texelDestino * t, 0).rgb;
    float3 b = T1.SampleLevel(Linear, uv + escolha.xy * texelDestino * (1.0 - t), 0).rgb;
    float melhor = dot(abs(a - b), float3(0.299, 0.587, 0.114)) - 0.01;
    [loop] for (int k = 1; k < 5; k++)
    {
        float4 outra = T2.Load(int3(clamp(m + VIZ[k], int2(0, 0), tamanhoEscolha - 1), 0));
        float3 ao = T0.SampleLevel(Linear, uv - outra.xy * texelDestino * t, 0).rgb;
        float3 bo = T1.SampleLevel(Linear, uv + outra.xy * texelDestino * (1.0 - t), 0).rgb;
        float erro = dot(abs(ao - bo), float3(0.299, 0.587, 0.114));
        if (erro < melhor)
        {
            melhor = erro;
            escolha = outra;
            a = ao;
            b = bo;
        }
    }
    float recuo = saturate((escolha.z - limiarErro) / faixaErro);
    float3 umLado = t < 0.5 ? a : b;
    float3 cor = lerp(lerp(a, b, t), umLado, recuo);

    // TRAVA: discordância grande demais quer dizer que o movimento não foi
    // achado (giro rapidíssimo, troca de cena, explosão). Aí o pixel mostra o
    // quadro real mais próximo NO LUGAR — no pior caso, um quadro repetido.
    // Nunca uma imagem derretida.
    float3 real = t < 0.5 ? anteriorNoLugar : atualNoLugar;
    float perdido = saturate((escolha.z - (limiarErro + faixaErro)) / faixaErro);
    return float4(Sair(lerp(cor, real, perdido)), 1);
}

// O quadro real, sem mexer.
float4 Copia(Saida e) : SV_Target
{
    float3 cor = T1.SampleLevel(Linear, e.pos.xy * texelDestino, 0).rgb;
    // Diagnóstico: lambda negativo inverte as cores, para provar que a
    // sobreposição está na tela.
    return float4(Sair(lambda < 0 ? 1.0 - cor : cor), 1);
}

// COMPOSIÇÃO SEM BLOCOS. A `Final` escolhe, para cada pedaço de 2×2, o vetor de um dos blocos 8×8 vizinhos: com a
// câmera girando, pedaços lado a lado escolhem movimentos diferentes e a imagem vira quadradinhos (o "pixelado" visto
// no FiveM). Aqui o vetor sai do campo já limpo pela mediana (T2, em 1/8), interpolado pixel a pixel: o movimento muda
// suave de um ponto para o outro. Onde os dois reais discordam (borda de quem se move, fundo que aparece), sai UM lado
// só, o mais próximo no tempo, DESLOCADO pelo vetor: nunca o real no lugar, que remendava a imagem com pedaços parados.
// T0 = anterior, T1 = atual, T2 = vetores suaves (1/8), T3 = interface (Mascara).
float4 FinalSuave(Saida e) : SV_Target
{
    float2 uv = e.pos.xy * texelDestino;
    float3 anteriorNoLugar = T0.SampleLevel(Linear, uv, 0).rgb;
    float3 atualNoLugar = T1.SampleLevel(Linear, uv, 0).rgb;
    if (T3.Load(int3(int2(e.pos.xy) / 2, 0)).r > 0.5)
        return float4(Sair(t < 0.5 ? anteriorNoLugar : atualNoLugar), 1);
    if (dot(abs(anteriorNoLugar - atualNoLugar), float3(0.3333, 0.3333, 0.3333)) < 0.004)
        return float4(Sair(atualNoLugar), 1);

    float2 v = T2.SampleLevel(Linear, uv, 0).xy;
    float3 a = T0.SampleLevel(Linear, uv - v * texelDestino * t, 0).rgb;
    float3 b = T1.SampleLevel(Linear, uv + v * texelDestino * (1.0 - t), 0).rgb;
    float erro = dot(abs(a - b), PESOS_LUMA);
    float w = saturate((erro - limiarErro) / faixaErro);
    float3 umLado = t < 0.5 ? a : b;
    return float4(Sair(lerp(lerp(a, b, t), umLado, w)), 1);
}

// ---------------------------------------------------------------------------
// FLUXO ÓPTICO POR BUSCA COMPLETA — o método do fluxo óptico do AMD FidelityFX
// (FSR 3, ffx_opticalflow_compute_optical_flow_v5.h e
// ffx_opticalflow_filter_optical_flow_v5.h), reescrito para Direct3D 11 e para
// a busca SIMÉTRICA do gerador (anterior(p − u) contra atual(p + u)).
//
// Copyright (C) 2026 Advanced Micro Devices, Inc.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files(the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and /or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions :
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
//
// A busca antiga testava poucos candidatos (±1 no nível fino) e chutava no
// cenário liso. Aqui, em cada nível da pirâmide, cada bloco 8×8 testa TODOS os
// 289 deslocamentos de ±8 pixels em volta da previsão do nível de cima, e a
// previsão é o melhor entre o "pai" e três vizinhos. No último nível o bloco
// só se move se mover for melhor que ficar parado.
//
// Os níveis andam de 2 em 2 (1/16 → 1/8 → 1/4 → 1/2). T0/T1 = luma do nível
// (anterior/atual), T2 = vetores do nível de cima (no primeiro, o movimento da
// câmera, 1×1). tamanhoOrigem = tamanho do nível; tamanhoAux = tamanho de T2;
// lambda = escala do nível (pixels da imagem por pixel do nível);
// limiarErro > 0.5 = último nível.
// ---------------------------------------------------------------------------
#define BLOCO 8
#define RAIO 8
#define JANELA (BLOCO + 2 * RAIO)
#define CANDIDATOS ((2 * RAIO + 1) * (2 * RAIO + 1))

RWTexture2D<float4> SaidaDoFluxo : register(u0);
groupshared float janelaA[JANELA * JANELA];
groupshared float janelaB[JANELA * JANELA];
groupshared float sadDaThread[64];
groupshared int idxDaThread[64];
groupshared int2 previsao;
groupshared float sadDaPrevisao[6];
groupshared int2 candidato[6];

float LerA(int2 p) { return T0.Load(int3(clamp(p, int2(0, 0), tamanhoOrigem - 1), 0)).r; }
float LerB(int2 p) { return T1.Load(int3(clamp(p, int2(0, 0), tamanhoOrigem - 1), 0)).r; }

// SAD do bloco inteiro com o meio-deslocamento u, direto das texturas.
float SadDoBloco(int2 canto, int2 u)
{
    float s = 0;
    [loop] for (int y = 0; y < BLOCO; y++)
        [loop] for (int x = 0; x < BLOCO; x++)
        {
            int2 p = canto + int2(x, y);
            s += abs(LerA(p - u) - LerB(p + u));
        }
    return s;
}

// SAD a partir das janelas: d = deslocamento em volta da previsão.
float SadDaJanela(int2 d)
{
    float s = 0;
    [loop] for (int y = 0; y < BLOCO; y++)
        [loop] for (int x = 0; x < BLOCO; x++)
            s += abs(janelaA[(RAIO + y - d.y) * JANELA + RAIO + x - d.x] - janelaB[(RAIO + y + d.y) * JANELA + RAIO + x + d.x]);
    return s;
}

[numthreads(8, 8, 1)]
void Busca(uint3 grupo : SV_GroupID, uint indice : SV_GroupIndex)
{
    int2 bloco = int2(grupo.xy);
    int2 canto = bloco * BLOCO;
    float escala = lambda;

    // 1. A previsão: o melhor (pela diferença do bloco) entre o pai, três vizinhos dele e a mediana vetorial dos 3×3
    //    pais (o "scale" e o "filter" do AMD), e o parado. A mediana é o que segura a textura repetida: um pai que casou com a tábua
    //    errada perde para o movimento da vizinhança.
    if (indice < 6)
    {
        int2 pai = bloco / 2;
        int2 lado = int2((bloco.x & 1) == 0 ? -1 : 1, (bloco.y & 1) == 0 ? -1 : 1);
        float2 v;
        if (indice == 5)
        {
            v = float2(0, 0);
        }
        else if (indice < 4)
        {
            int2 desvio = indice == 0 ? int2(0, 0) : indice == 1 ? int2(lado.x, 0) : indice == 2 ? int2(0, lado.y) : lado;
            v = T2.Load(int3(clamp(pai + desvio, int2(0, 0), tamanhoAux - 1), 0)).xy;
        }
        else
        {
            float2 viz[9];
            int n = 0;
            [unroll] for (int dy = -1; dy <= 1; dy++)
                [unroll] for (int dx = -1; dx <= 1; dx++)
                    viz[n++] = T2.Load(int3(clamp(pai + int2(dx, dy), int2(0, 0), tamanhoAux - 1), 0)).xy;
            float menor = 1e9;
            v = viz[4];
            [unroll] for (int a = 0; a < 9; a++)
            {
                float soma = 0;
                [unroll] for (int b = 0; b < 9; b++) soma += length(viz[a] - viz[b]);
                if (soma < menor) { menor = soma; v = viz[a]; }
            }
        }
        // Vetor total em pixels da imagem → meio-deslocamento em pixels deste nível.
        int2 u = int2(round(v / (2.0 * escala)));
        candidato[indice] = u;
        // A mediana ganha empate (estabilidade).
        sadDaPrevisao[indice] = SadDoBloco(canto, u) - (indice == 4 ? ajusteFluxo.y : 0.0);
        // Vantagem do parado (como o "chão liso"): um vetor só ganha dele se for claramente melhor.
        if (indice == 5) sadDaPrevisao[indice] = sadDaPrevisao[indice] * 0.85 - 0.08;
    }
    GroupMemoryBarrierWithGroupSync();
    if (indice == 0)
    {
        int melhor = 4;
        [unroll] for (int k = 0; k < 6; k++)
            if (k != 4 && sadDaPrevisao[k] < sadDaPrevisao[melhor]) melhor = k;
        previsao = candidato[melhor];
    }
    GroupMemoryBarrierWithGroupSync();

    // 2. As janelas de busca, uma vez para o grupo inteiro.
    int2 u0 = previsao;
    for (uint i = indice; i < JANELA * JANELA; i += 64)
    {
        int2 q = int2(i % JANELA, i / JANELA) - RAIO;
        janelaA[i] = LerA(canto + q - u0);
        janelaB[i] = LerB(canto + q + u0);
    }
    GroupMemoryBarrierWithGroupSync();

    // 3. Busca completa: 289 candidatos divididos pelas 64 threads.
    float melhorSad = 1e9;
    int melhorIdx = (CANDIDATOS - 1) / 2;
    for (uint k = indice; k < CANDIDATOS; k += 64)
    {
        int2 d = int2(int(k % (2 * RAIO + 1)), int(k / (2 * RAIO + 1))) - RAIO;
        // Desempate: perto da previsão ganha (ajusteFluxo.x por passo; ajusteFluxo.y = vantagem da mediana).
        float s = SadDaJanela(d) + ajusteFluxo.x * (abs(d.x) + abs(d.y));
        if (s < melhorSad) { melhorSad = s; melhorIdx = int(k); }
    }
    sadDaThread[indice] = melhorSad;
    idxDaThread[indice] = melhorIdx;
    GroupMemoryBarrierWithGroupSync();
    [unroll] for (uint passo = 32; passo > 0; passo >>= 1)
    {
        if (indice < passo && sadDaThread[indice + passo] < sadDaThread[indice])
        {
            sadDaThread[indice] = sadDaThread[indice + passo];
            idxDaThread[indice] = idxDaThread[indice + passo];
        }
        GroupMemoryBarrierWithGroupSync();
    }

    if (indice == 0)
    {
        int k = idxDaThread[0];
        int2 d = int2(k % (2 * RAIO + 1), k / (2 * RAIO + 1)) - RAIO;
        float s0 = sadDaThread[0];

        // Fração de pixel: parábola pelas vizinhas, quando estão dentro da janela.
        float2 fracao = float2(0, 0);
        if (abs(d.x) < RAIO && abs(d.y) < RAIO)
        {
            float sxm = SadDaJanela(d - int2(1, 0)), sxp = SadDaJanela(d + int2(1, 0));
            float sym = SadDaJanela(d - int2(0, 1)), syp = SadDaJanela(d + int2(0, 1));
            float cx = sxm + sxp - 2.0 * s0, cy = sym + syp - 2.0 * s0;
            fracao.x = cx > 1e-4 ? clamp(0.5 * (sxm - sxp) / cx, -0.5, 0.5) : 0.0;
            fracao.y = cy > 1e-4 ? clamp(0.5 * (sym - syp) / cy, -0.5, 0.5) : 0.0;
        }
        float2 u = float2(u0 + d) + fracao;

        // Último nível: parado vence o empate (o personagem e o HUD, em terceira pessoa).
        // Como o "chão liso" da busca antiga: só sai do parado se for CLARAMENTE melhor (areia, céu e asfalto empatam).
        if (limiarErro > 0.5 && s0 > SadDoBloco(canto, int2(0, 0)) * 0.85 - 0.08)
            u = float2(0, 0);

        // Vetor total (2u), em pixels da imagem.
        SaidaDoFluxo[bloco] = float4(u * 2.0 * escala, s0, 1);
    }
}

// Luma em meia resolução: média 2×2 da imagem.
float4 LumaMeia(Saida e) : SV_Target
{
    float2 centro = (floor(e.pos.xy) * 2.0 + 1.0) * texelOrigem;
    float3 c = T0.SampleLevel(Linear, centro, 0).rgb;
    // A textura de origem já em luma tem y no canal r.
    return float4(tamanhoAux.x == 1 ? c.r : dot(c, PESOS_LUMA), 0, 0, 1);
}

// Do campo do último nível (blocos de 16 px) para a grade de 1/8 que a Escolha usa (o "scale" do AMD). Com blocos de
// 16 px a cabeça do personagem e o cenário atrás dela caíam no mesmo bloco, e o rosto saía rasgado. Cada célula de 8 px
// escolhe, pela diferença dos SEUS pixels, entre os vetores dos 3×3 blocos em volta, a mediana deles e o parado.
// T0 = campo do último nível (tamanhoAux = blocos dele), T1/T2 = luma em 1/2 (anterior/atual; tamanhoOrigem, texelOrigem).
float SadDaCelula(int2 c, float2 v)
{
    // v em pixels da imagem; o meio-deslocamento em pixels de 1/2 é v / 4.
    float2 u = v * 0.25;
    float s = 0;
    [unroll] for (int y = -1; y <= 4; y++)
        [unroll] for (int x = -1; x <= 4; x++)
        {
            float2 centro = float2(c + int2(x, y)) + 0.5;
            s += abs(T1.SampleLevel(Linear, (centro - u) * texelOrigem, 0).r - T2.SampleLevel(Linear, (centro + u) * texelOrigem, 0).r);
        }
    return s;
}

float4 Amplia(Saida e) : SV_Target
{
    int2 q = int2(e.pos.xy);
    int2 c = q * 4;
    int2 p = q / 2;
    float2 v[9];
    int i = 0;
    [unroll] for (int dy = -1; dy <= 1; dy++)
        [unroll] for (int dx = -1; dx <= 1; dx++)
            v[i++] = T0.Load(int3(clamp(p + int2(dx, dy), int2(0, 0), tamanhoAux - 1), 0)).xy;
    float menor = 1e9;
    float2 mediana = v[4];
    [unroll] for (int a = 0; a < 9; a++)
    {
        float soma = 0;
        [unroll] for (int b = 0; b < 9; b++) soma += length(v[a] - v[b]);
        if (soma < menor) { menor = soma; mediana = v[a]; }
    }
    // A mediana ganha empate; o parado só perde se o vetor for claramente melhor (36 pontos por célula).
    float melhor = SadDaCelula(c, mediana) - 0.01;
    float2 escolhido = mediana;
    [unroll] for (int k = 0; k < 9; k++)
    {
        float sk = SadDaCelula(c, v[k]);
        if (sk < melhor) { melhor = sk; escolhido = v[k]; }
    }
    if (SadDaCelula(c, float2(0, 0)) * 0.85 - 0.045 <= melhor) escolhido = float2(0, 0);
    return float4(escolhido, 0, 1);
}

// ---------------------------------------------------------------------------
// FLUXO DENSO POR BUSCA INVERSA (DIS — Kroeger, Timofte, Dai e Van Gool, "Fast
// Optical Flow using Dense Inverse Search", ECCV 2016), na forma SIMÉTRICA do
// gerador: para um ponto x do quadro do meio, B(x + u) deve ser igual a A(x − u).
//
// Por que não blocos: a busca por blocos só testa deslocamentos inteiros e dá
// UM vetor para o bloco inteiro; na borda do personagem metade do bloco é
// cenário, e o quadro gerado sai em quadradinhos. Aqui, em cada nível da
// pirâmide (1/16 → 1/8 → 1/4 → 1/2):
//   1. DisPedaco — pedaços 8×8 a cada 4 pixels (sobrepostos). Cada um parte do
//      melhor entre o campo do nível de cima, o parado e a câmera, e refina por
//      Gauss-Newton (precisão abaixo do pixel). O amortecimento segura o chão
//      liso no vetor de partida, em vez de inventar movimento.
//   2. DisDenso — cada pixel do nível recebe a média dos vetores dos pedaços
//      que o cobrem, pesada pelo quanto cada vetor acerta NAQUELE pixel: o
//      campo sai liso, mas a borda de quem se move continua nítida.
//
// Texturas de luma com gradiente (Gradiente): r = luma, g/b = derivadas.
// ---------------------------------------------------------------------------
#define DIS_PEDACO 8
#define DIS_PASSO 4

// Luma e gradiente do nível (r, g, b) em x, em pixels do nível (centro do pixel em +0,5).
float3 LA(float2 x) { return T0.SampleLevel(Linear, x * texelOrigem, 0).rgb; }
float3 LB(float2 x) { return T1.SampleLevel(Linear, x * texelOrigem, 0).rgb; }

// A direção da busca (`rotacao`, que só o passe Entrada usa para outra coisa): 0 = simétrica, A(x − u) contra B(x + u),
// o campo no quadro do meio; 1 = de ida, A(x) contra B(x + 2u), o campo nos pixels de A; 2 = de volta, A(x − 2u) contra
// B(x), o campo nos pixels de B. O vetor guardado é sempre o total (2u), no sentido do movimento de A para B.
float2 PesosDaDirecao()
{
    if (rotacao > 0.5 && rotacao < 1.5) return float2(0.0, 2.0);
    if (rotacao > 1.5) return float2(2.0, 0.0);
    return float2(1.0, 1.0);
}

float4 Gradiente(Saida e) : SV_Target
{
    int2 p = int2(e.pos.xy);
    float c = Ler0(p);
    float gx = 0.5 * (Ler0(p + int2(1, 0)) - Ler0(p - int2(1, 0)));
    float gy = 0.5 * (Ler0(p + int2(0, 1)) - Ler0(p - int2(0, 1)));
    return float4(c, gx, gy, 1);
}

// Um grupo de 64 threads por pedaço, uma por pixel, e as somas do pedaço em memória compartilhada (como no fluxo do
// AMD). Com uma thread por pedaço percorrendo os 64 pixels em série, a placa passava o tempo esperando memória: o nível
// de 1/4 sozinho custava 1,6 ms numa GTX 1650.
groupshared float4 somaDoGrupo[64];
groupshared float somaDoGrupo2[64];

// Soma dos 64 valores (v) e (w) do grupo. Todas as threads recebem o total.
float4 SomarNoGrupo(uint i, float4 v, out float total2, float w)
{
    GroupMemoryBarrierWithGroupSync();
    somaDoGrupo[i] = v;
    somaDoGrupo2[i] = w;
    GroupMemoryBarrierWithGroupSync();
    [unroll] for (uint passo = 32; passo > 0; passo >>= 1)
    {
        if (i < passo)
        {
            somaDoGrupo[i] += somaDoGrupo[i + passo];
            somaDoGrupo2[i] += somaDoGrupo2[i + passo];
        }
        GroupMemoryBarrierWithGroupSync();
    }
    total2 = somaDoGrupo2[0];
    return somaDoGrupo[0];
}

float SomarErro(uint i, float2 q, float2 u)
{
    float2 k = PesosDaDirecao();
    float d = LB(q + k.y * u).r - LA(q - k.x * u).r;
    float nada;
    return SomarNoGrupo(i, float4(d * d, 0, 0, 0), nada, 0).x;
}

// T0/T1 = luma com gradiente do nível (anterior/atual; tamanhoOrigem, texelOrigem).
// T2 = campo denso do nível de cima (no primeiro, a câmera 1×1). T3 = câmera (1×1).
// lambda = escala do nível; limiarErro = raio da busca inteira antes do refino (0 = sem);
// faixaErro = iterações; ajusteFluxo.x = amortecimento, ajusteFluxo.y = folga do parado.
[numthreads(8, 8, 1)]
void DisPedaco(uint3 grupo : SV_GroupID, uint3 local : SV_GroupThreadID, uint i : SV_GroupIndex)
{
    int2 n = int2((tamanhoOrigem - DIS_PEDACO + DIS_PASSO - 1) / DIS_PASSO + 1);
    if (any(int2(grupo.xy) >= n)) return;
    float2 canto = float2(grupo.xy) * DIS_PASSO;
    float2 q = canto + float2(local.xy) + 0.5;
    float escala = lambda;
    float2 centro = canto + DIS_PEDACO * 0.5;

    // Partida: o campo de cima no centro do pedaço, o parado ou a câmera — o que errar menos.
    float2 u = T2.SampleLevel(Linear, centro * texelOrigem, 0).xy / (2.0 * escala);
    float2 camera = T3.Load(int3(0, 0, 0)).xy / (2.0 * escala);
    float melhor = SomarErro(i, q, u);
    // O parado ganha com folga (personagem e HUD em terceira pessoa): sem a margem absoluta, numa máscara escura e
    // lisa o parado e a câmera erram quase igual, a câmera vencia e o rosto saía com pedaços da areia atrás.
    float erroParado = SomarErro(i, q, float2(0, 0));
    if (erroParado * 0.85 - ajusteFluxo.y < melhor) { melhor = erroParado * 0.85 - ajusteFluxo.y; u = float2(0, 0); }
    float erroCamera = SomarErro(i, q, camera);
    if (erroCamera < melhor) { melhor = erroCamera; u = camera; }

    // PROPAGAÇÃO ESPACIAL (o DIS do OpenCV): o movimento dos quatro pedaços vizinhos, no nível de cima, também é
    // partida. Na borda de quem se move, o centro do pedaço cai no movimento errado e o vizinho tem o certo.
    if (extra.x > 0.5)
    {
        [unroll] for (int k = 0; k < 4; k++)
        {
            float2 d = k == 0 ? float2(DIS_PEDACO, 0) : k == 1 ? float2(-DIS_PEDACO, 0) : k == 2 ? float2(0, DIS_PEDACO) : float2(0, -DIS_PEDACO);
            float2 c = T2.SampleLevel(Linear, (centro + d) * texelOrigem, 0).xy / (2.0 * escala);
            float e = SomarErro(i, q, c);
            if (e < melhor) { melhor = e; u = c; }
        }
    }

    // Nos níveis de cima, uma busca inteira em volta da partida: o refino só converge até ~1 pixel.
    int raio = int(limiarErro);
    if (raio > 0)
    {
        float2 base = round(u);
        [loop] for (int dy = -raio; dy <= raio; dy++)
            [loop] for (int dx = -raio; dx <= raio; dx++)
            {
                float2 c = base + float2(dx, dy);
                float e = SomarErro(i, q, c) * (1.0 + 0.01 * (abs(dx) + abs(dy)));
                if (e < melhor) { melhor = e; u = c; }
            }
    }

    // Refino de Gauss-Newton simétrico: r = B(x + u) − A(x − u), dr/du = ∇B + ∇A.
    float2 inicio = u;
    float erroInicio = all(u == float2(0, 0)) ? erroParado : SomarErro(i, q, u);
    int iteracoes = int(faixaErro);
    float amortecimento = ajusteFluxo.x;
    [loop] for (int it = 0; it < iteracoes; it++)
    {
        float2 k = PesosDaDirecao();
        float3 a = LA(q - k.x * u);
        float3 b = LB(q + k.y * u);
        float r = b.r - a.r;
        float2 j = k.y * b.gb + k.x * a.gb;
        float gy;
        float4 h = SomarNoGrupo(i, float4(j.x * j.x, j.x * j.y, j.y * j.y, j.x * r), gy, j.y * r);
        float h11 = h.x + amortecimento, h12 = h.y, h22 = h.z;
        float2 g = float2(h.w, gy);
        float det = h11 * h22 - h12 * h12;
        if (det < 1e-10) break;
        float2 du = -float2(h22 * g.x - h12 * g.y, h11 * g.y - h12 * g.x) / det;
        du = clamp(du, -1.0, 1.0);
        u += du;
        if (dot(du, du) < 0.0001) break;
    }
    // Se o refino piorou (saiu do vale), volta à partida. E o parado de novo, com a mesma folga.
    float erroFinal = SomarErro(i, q, u);
    if (erroFinal > erroInicio) { u = inicio; erroFinal = erroInicio; }
    if (erroParado * 0.85 - ajusteFluxo.y <= erroFinal) u = float2(0, 0);

    // Vetor total (2u), em pixels da imagem.
    if (i == 0) SaidaDoFluxo[grupo.xy] = float4(u * 2.0 * escala, 0, 1);
}

// Campo denso do nível. T0/T1 = luma com gradiente; T2 = vetores dos pedaços (tamanhoAux = quantos).
float4 DisDenso(Saida e) : SV_Target
{
    int2 p = int2(e.pos.xy);
    float2 q = float2(p) + 0.5;
    float escala = lambda;
    int2 lo = max(p - (DIS_PEDACO - DIS_PASSO), int2(0, 0)) / DIS_PASSO;
    int2 hi = min(p / DIS_PASSO, tamanhoAux - 1);
    float2 soma = float2(0, 0);
    float pesos = 0;
    [loop] for (int y = lo.y; y <= hi.y; y++)
        [loop] for (int x = lo.x; x <= hi.x; x++)
        {
            float2 v = T2.Load(int3(x, y, 0)).xy;
            float2 u = v / (2.0 * escala);
            float2 k = PesosDaDirecao();
            float d = abs(LB(q + k.y * u).r - LA(q - k.x * u).r);
            float w = 1.0 / max(d, 0.01);
            soma += w * v;
            pesos += w;
        }
    float2 v = pesos > 0 ? soma / pesos : float2(0, 0);
    return float4(v, 0, 1);
}

// Composição com o campo denso. T0 = anterior, T1 = atual, T2 = campo denso (1/2), T3 = interface.
// Cada pixel testa o vetor dele e os de quatro pontos a 8 pixels: na borda de quem se move, o fundo que aparece pega
// o vetor do fundo, e não o do personagem. O erro é medido numa cruz de 5 pontos, para vizinhos concordarem.
float ErroNaCruz(float2 uv, float2 v)
{
    float2 o = texelDestino;
    float2 da = -v * texelDestino * t;
    float2 db = v * texelDestino * (1.0 - t);
    float erro = 0;
    [unroll] for (int k = 0; k < 5; k++)
    {
        float2 d = k == 0 ? float2(0, 0) : k == 1 ? float2(1.5, 0) : k == 2 ? float2(-1.5, 0) : k == 3 ? float2(0, 1.5) : float2(0, -1.5);
        float3 a = T0.SampleLevel(Linear, uv + d * o + da, 0).rgb;
        float3 b = T1.SampleLevel(Linear, uv + d * o + db, 0).rgb;
        erro += dot(abs(a - b), PESOS_LUMA);
    }
    return erro * 0.2;
}

float4 FinalDenso(Saida e) : SV_Target
{
    float2 uv = e.pos.xy * texelDestino;
    float3 anteriorNoLugar = T0.SampleLevel(Linear, uv, 0).rgb;
    float3 atualNoLugar = T1.SampleLevel(Linear, uv, 0).rgb;
    if (T3.Load(int3(int2(e.pos.xy) / 2, 0)).r > 0.5)
        return float4(Sair(t < 0.5 ? anteriorNoLugar : atualNoLugar), 1);
    if (dot(abs(anteriorNoLugar - atualNoLugar), float3(0.3333, 0.3333, 0.3333)) < 0.004)
        return float4(Sair(atualNoLugar), 1);

    float2 v = T2.SampleLevel(Linear, uv, 0).xy;
    float melhor = ErroNaCruz(uv, v) - 0.005;
    [unroll] for (int k = 0; k < 5; k++)
    {
        float2 d = (k == 0 ? float2(8, 0) : k == 1 ? float2(-8, 0) : k == 2 ? float2(0, 8) : float2(0, -8)) * texelDestino;
        // O último candidato é o parado.
        float2 outro = k == 4 ? float2(0, 0) : T2.SampleLevel(Linear, uv + d, 0).xy;
        float erro = ErroNaCruz(uv, outro);
        if (erro < melhor) { melhor = erro; v = outro; }
    }
    float3 a = T0.SampleLevel(Linear, uv - v * texelDestino * t, 0).rgb;
    float3 b = T1.SampleLevel(Linear, uv + v * texelDestino * (1.0 - t), 0).rgb;
    float erro = max(melhor, 0.0);
    float recuo = saturate((erro - limiarErro) / faixaErro);
    float3 umLado = t < 0.5 ? a : b;
    float3 cor = lerp(lerp(a, b, t), umLado, recuo);
    float3 real = t < 0.5 ? anteriorNoLugar : atualNoLugar;
    float perdido = saturate((erro - (limiarErro + faixaErro)) / faixaErro);
    return float4(Sair(lerp(cor, real, perdido)), 1);
}

// ---------------------------------------------------------------------------
// OCLUSÃO PELOS VETORES DE IDA E VOLTA (a "máscara de cobre/descobre" do SVP e as duas máscaras de desoclusão do FSR 3).
// Um pedaço de A que tem par em B: o vetor de ida dele, f(p), leva a q = p + f(p), e o vetor de volta em q, b(q), tem de
// ser o mesmo. Se não for, aquele pedaço de A some em B (foi coberto): o quadro gerado não pode tirar cor de B ali. O
// mesmo ao contrário para o que aparece em B (descoberto). T0 = campo de ida (pixels de A), T1 = campo de volta (pixels
// de B), na mesma grade; texelDestino = 1 / tamanho da imagem. Saída: x = o pixel de A tem par em B, y = o de B tem em A.
// ---------------------------------------------------------------------------
float4 Consistencia(Saida e) : SV_Target
{
    float2 uv = (floor(e.pos.xy) + 0.5) / float2(tamanhoOrigem);
    float2 f = T0.SampleLevel(Linear, uv, 0).xy;
    float2 bq = T1.SampleLevel(Linear, uv + f * texelDestino, 0).xy;
    float2 b = T1.SampleLevel(Linear, uv, 0).xy;
    float2 fq = T0.SampleLevel(Linear, uv - b * texelDestino, 0).xy;
    // Tolerância: 3 px mais 15% do movimento (o campo é de 1/8, e vetor longo erra mais em pixels).
    float errA = length(f - bq) / (3.0 + 0.15 * length(f));
    float errB = length(b - fq) / (3.0 + 0.15 * length(b));
    return float4(saturate(2.0 - errA), saturate(2.0 - errB), 0, 1);
}

float3 Mediana3(float3 a, float3 b, float3 c)
{
    return max(min(a, b), min(max(a, b), c));
}

// COMPOSIÇÃO COM OCLUSÃO. T0 = anterior, T1 = atual, T2 = campo do meio (DIS simétrico), T3 = interface, T4 = a
// consistência acima. Escolhe o vetor como a FinalDenso (o do pixel ou de um vizinho a 8 px, pela cruz de erro) e então:
//   - só o lado que existe: o pedaço de A coberto em B sai só de A, o que aparece em B sai só de B;
//   - os dois existem: a MEDIANA DINÂMICA (SVP, modo 13) entre A deslocado, B deslocado e a mistura parada — quando os
//     dois deslocados concordam, é eles; quando um vetor erra e eles discordam, sai o valor do meio, em vez do pixel
//     quebrado que um dos lados trazia;
//   - nenhum dos dois: o real mais próximo no tempo, deslocado (como antes).
// TROCA DE CENA (a `scd` do FSR 3, aqui pela própria consistência): o último mip de T4 é a fração da imagem em que os
// dois reais têm par um no outro. Medido nos quadros gravados do FiveM: pares seguidos casam de 55% a 100%; um corte
// de câmera (outra cena inteira) casa de 2% a 13%. Abaixo de 25% não há o que interpolar — sai o real mais próximo;
// de 25% a 40% a mistura vai para o real aos poucos. Lido na placa: a CPU não espera nada.
static const float CORTE_TOTAL = 0.25;
static const float CORTE_FAIXA = 0.15;

float4 FinalBidirecional(Saida e) : SV_Target
{
    float2 uv = e.pos.xy * texelDestino;
    float3 anteriorNoLugar = T0.SampleLevel(Linear, uv, 0).rgb;
    float3 atualNoLugar = T1.SampleLevel(Linear, uv, 0).rgb;
    if (T3.Load(int3(int2(e.pos.xy) / 2, 0)).r > 0.5)
        return float4(Sair(t < 0.5 ? anteriorNoLugar : atualNoLugar), 1);
    if (dot(abs(anteriorNoLugar - atualNoLugar), float3(0.3333, 0.3333, 0.3333)) < 0.004)
        return float4(Sair(atualNoLugar), 1);
    float2 casou = T4.SampleLevel(Linear, float2(0.5, 0.5), 16.0).xy;
    float corte = saturate((CORTE_TOTAL + CORTE_FAIXA - min(casou.x, casou.y)) / CORTE_FAIXA);
    if (corte >= 1.0)
        return float4(Sair(t < 0.5 ? anteriorNoLugar : atualNoLugar), 1);

    float2 v = T2.SampleLevel(Linear, uv, 0).xy;
    float melhor = ErroNaCruz(uv, v) - 0.005;
    [unroll] for (int k = 0; k < 5; k++)
    {
        float2 d = (k == 0 ? float2(8, 0) : k == 1 ? float2(-8, 0) : k == 2 ? float2(0, 8) : float2(0, -8)) * texelDestino;
        float2 outro = k == 4 ? float2(0, 0) : T2.SampleLevel(Linear, uv + d, 0).xy;
        float erro = ErroNaCruz(uv, outro);
        if (erro < melhor) { melhor = erro; v = outro; }
    }
    float2 pa = uv - v * texelDestino * t;
    float2 pb = uv + v * texelDestino * (1.0 - t);
    float3 a = T0.SampleLevel(Linear, pa, 0).rgb;
    float3 b = T1.SampleLevel(Linear, pb, 0).rgb;
    float existeA = T4.SampleLevel(Linear, pa, 0).x;
    float existeB = T4.SampleLevel(Linear, pb, 0).y;

    float wA = (1.0 - t) * existeA;
    float wB = t * existeB;
    float3 umLado = t < 0.5 ? a : b;
    float3 cor;
    if (wA + wB < 0.02)
    {
        cor = umLado;
    }
    else
    {
        float3 pelaOclusao = (a * wA + b * wB) / (wA + wB);
        float3 parada = lerp(anteriorNoLugar, atualNoLugar, t);
        float3 mediana = Mediana3(a, b, parada);
        // A mediana só onde os dois lados existem; onde um some, vale só o outro.
        cor = lerp(pelaOclusao, mediana, existeA * existeB);
    }

    float erro = max(melhor, 0.0);
    float3 real = t < 0.5 ? anteriorNoLugar : atualNoLugar;
    float perdido = saturate((erro - (limiarErro + faixaErro)) / faixaErro);
    return float4(Sair(lerp(cor, real, max(perdido, corte))), 1);
}
