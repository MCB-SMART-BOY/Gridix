# Nix Flake Install Guide | Nix Flake 安装指南

This page focuses on installing and running Gridix through Nix Flake.  
本页专门说明如何通过 Nix Flake 安装和运行 Gridix。

## 1. Prerequisite | 前置条件

Enable Nix experimental features:
启用 Nix 实验特性：

```bash
nix --extra-experimental-features "nix-command flakes" --version
```

## 2. Run Without Install | 不安装直接运行

```bash
nix run github:MCB-SMART-BOY/Gridix
```

## 3. Install To Profile | 安装到用户环境

```bash
nix profile install github:MCB-SMART-BOY/Gridix
```

After install:
安装后运行：

```bash
gridix
```

## 4. Use Specific Version | 安装指定版本

```bash
nix run github:MCB-SMART-BOY/Gridix/v4.1.0
nix profile install github:MCB-SMART-BOY/Gridix/v4.1.0
```

## 5. Build Locally | 本地构建

```bash
git clone https://github.com/MCB-SMART-BOY/Gridix.git
cd Gridix
nix build .#gridix
./result/bin/gridix
```

## 6. Development Shell | 开发环境

```bash
nix develop
```

## 7. Use As Overlay | 作为 overlay 使用

In another flake:
在其他 flake 中引用：

```nix
{
  inputs.gridix.url = "github:MCB-SMART-BOY/Gridix";

  outputs = { self, nixpkgs, gridix, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ gridix.overlays.default ];
      };
    in {
      packages.${system}.default = pkgs.gridix;
    };
}
```
