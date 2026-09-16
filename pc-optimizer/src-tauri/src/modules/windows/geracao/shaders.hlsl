// ---------------------------------------------------------------------------
// GERADOR DE QUADROS DO OTIMIZA — shaders
//
// Tudo roda na placa de vídeo, em passes de tela cheia:
//
//   1. Luma      — imagem → luminância em 1/4 e depois 1/16 da resolução
//   2. Grossa    — busca de movimento em 1/16, raio ±6 (±96 px na imagem)
//   3. Fina      — refina em 1/4, com os vetores grossos da vizinhança como
//                  candidatos, na grade de 1/8
//   4. Suave     — mediana vetorial 3×3: tira vetor isolado errado
//   5. Final     — cada pixel do quadro gerado busca a cor seguindo o vetor
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
};

Texture2D<float4> T0 : register(t0);
Texture2D<float4> T1 : register(t1);
Texture2D<float4> T2 : register(t2);
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

// Busca grossa em 1/16. T0/T1 = luma 1/16 anterior/atual.
float4 Grossa(Saida e) : SV_Target
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
            sad += lambda * (abs(ux) + abs(uy));
            if (sad < melhor) { melhor = sad; escolhido = u; }
            if (ux == 0 && uy == 0) parado = sad;
        }
    }
    // PARADO VENCE EMPATE. Textura repetida (grade, cerca, piso) casa a um
    // período de distância quase tão bem quanto no lugar certo; sem esta
    // preferência, a imagem rasga em faixas.
    if (parado <= melhor * 1.15 + 0.02) escolhido = int2(0, 0);
    // Vetor total entre os dois reais: 2u em pixels de 1/16 → ×16 na imagem.
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

// Composição. T0 = anterior, T1 = atual (imagem inteira), T2 = vetores (1/8).
//
// CADA PIXEL ESCOLHE O PRÓPRIO VETOR. Um vetor por bloco de 8×8 erra na borda
// entre dois movimentos (objeto sobre fundo, personagem sobre cenário): metade
// do bloco anda de um jeito, metade de outro. Aqui o pixel testa os vetores dos
// blocos vizinhos em cruz e o vetor parado, e fica com aquele em que os dois quadros
// reais concordam NAQUELE pixel. Onde nenhum concorda, mistura os dois reais
// no lugar — um leve fantasma em vez de uma mancha inventada.
float4 Final(Saida e) : SV_Target
{
    float2 uv = e.pos.xy * texelDestino;
    int2 bloco = int2(e.pos.xy) / 8;

    float melhorErro = 1e9;
    float3 melhorCor = float3(0, 0, 0);
    static const int2 CRUZ[5] = { int2(0, 0), int2(1, 0), int2(-1, 0), int2(0, 1), int2(0, -1) };
    [loop] for (int k = 0; k < 6; k++)
    {
        float2 v = float2(0, 0);
        if (k < 5)
            v = T2.Load(int3(clamp(bloco + CRUZ[k], int2(0, 0), tamanhoAux - 1), 0)).xy;
        float2 vuv = v * texelDestino;
        float4 a = T0.SampleLevel(Linear, uv - vuv * t, 0);
        float4 b = T1.SampleLevel(Linear, uv + vuv * (1.0 - t), 0);
        float erro = dot(abs(a.rgb - b.rgb), float3(0.3333, 0.3333, 0.3333));
        // O vetor do próprio bloco ganha empate: estabilidade entre quadros.
        if (k == 0) erro -= 0.004;
        if (erro < melhorErro)
        {
            melhorErro = erro;
            melhorCor = lerp(a.rgb, b.rgb, t);
        }
    }

    float recuo = saturate((melhorErro - limiarErro) / faixaErro);
    float3 noLugar = lerp(T0.SampleLevel(Linear, uv, 0).rgb, T1.SampleLevel(Linear, uv, 0).rgb, t);
    return float4(lerp(melhorCor, noLugar, recuo), 1);
}

// O quadro real, sem mexer.
float4 Copia(Saida e) : SV_Target
{
    float3 cor = T1.SampleLevel(Linear, e.pos.xy * texelDestino, 0).rgb;
    // Diagnóstico: lambda negativo inverte as cores, para provar que a
    // sobreposição está na tela.
    return float4(lambda < 0 ? 1.0 - cor : cor, 1);
}
