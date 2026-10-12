// Laboratório E2: a Composition Swapchain (presentation.h, Windows 11 22000.194+) existe neste PC? Cria a fábrica de
// apresentação no dispositivo D3D11 (com o requisito da Microsoft: sem as otimizações internas de threads do driver)
// e pergunta se a apresentação com horário marcado é suportada.
#include <windows.h>
#include <d3d11.h>
#include <presentation.h>
#include <cstdio>

#pragma comment(lib, "d3d11.lib")
#pragma comment(lib, "dcomp.lib")

int main() {
    ID3D11Device* d3d = nullptr;
    HRESULT hr = D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
                                   D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_PREVENT_INTERNAL_THREADING_OPTIMIZATIONS,
                                   nullptr, 0, D3D11_SDK_VERSION, &d3d, nullptr, nullptr);
    if (FAILED(hr)) { std::printf("D3D11: 0x%08X\n", static_cast<unsigned>(hr)); return 1; }
    IPresentationFactory* fabrica = nullptr;
    hr = CreatePresentationFactory(d3d, __uuidof(IPresentationFactory), reinterpret_cast<void**>(&fabrica));
    if (FAILED(hr)) { std::printf("CreatePresentationFactory: 0x%08X\n", static_cast<unsigned>(hr)); return 2; }
    std::printf("apresentacao com horario suportada: %u\n", fabrica->IsPresentationSupported());
    std::printf("com scanout direto (independent flip): %u\n", fabrica->IsPresentationSupportedWithIndependentFlip());
    IPresentationManager* gerente = nullptr;
    hr = fabrica->CreatePresentationManager(&gerente);
    std::printf("gerente: 0x%08X\n", static_cast<unsigned>(hr));
    if (gerente) gerente->Release();
    fabrica->Release();
    d3d->Release();
    return 0;
}
