# AXIS — Documentação Completa

## Índice

1. [Visão Geral](#visão-geral)
2. [Arquitetura](#arquitetura)
3. [Instalação](#instalação)
4. [Configuração](#configuração)
5. [API Reference](#api-reference)
6. [Testes](#testes)
7. [Deploy](#deploy)

---

## Visão Geral

**AXIS** é o servidor central do ecossistema OmniHub, responsável por:

- **Autenticação centralizada** via JWT RS256 (emissão e validação de tokens)
- **Gestão de franquias e billing** (cobranças mensais, status de pagamento)
- **Coverage map** com PostGIS (verificação geoespacial de hubs)
- **Roaming entre hubs** (validação de motoristas em hubs externos)
- **RBAC** (controle de acesso baseado em papéis)
- **Sistema de bloqueios** (usuários e motoristas)

O AXIS é consumido pelos servidores de hub individuais (OmniHub Nodes), que delegam autenticação e operações de controle para ele.

---

## Arquitetura

### Camadas

```
┌─────────────────────────────────────┐
│         API Layer (Axum)            │
│  Handlers: auth, payments, hubs,    │
│  roaming, coverage, webhooks        │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│       Business Logic Layer          │
│  Services: PaymentService,          │
│  HubStatusService, BlockService,    │
│  NotificationService, AuthService,  │
│  PaymentGateway                     │
└──────────────┬──────────────────────┘
               │
┌──────────────▼──────────────────────┐
│       Data Layer (SQLx)             │
│  Models + Migrations                │
│  PostgreSQL + PostGIS               │
└─────────────────────────────────────┘
```

### Tecnologias

| Componente        | Tecnologia              |
|-------------------|-------------------------|
| Linguagem         | Rust (stable)           |
| Web Framework     | Axum 0.7                |
| Banco de Dados    | PostgreSQL 14+ + PostGIS|
| ORM/Query        | SQLx 0.7                |
| Autenticação      | JWT RS256 (jsonwebtoken) |
| Gateway Pagamento | Mercado Pago            |
| Async Runtime     | Tokio                   |

### Sistema de Status de Hub

O AXIS implementa degradação gradual de status com base nos dias de atraso de pagamento:

| Dias de Atraso | Status       | Impacto                        |
|----------------|--------------|--------------------------------|
| 0 – 7 dias     | `active`     | Nenhum — operação normal       |
| 8 – 10 dias    | `grace`      | Avisos enviados ao franqueado  |
| 11 – 14 dias   | `restricted` | Sem aceitação de novos motoristas |
| 15+ dias       | `suspended`  | Hub offline                    |

---

## Instalação

### Pré-requisitos

- Rust 1.70+
- PostgreSQL 14+ com extensão PostGIS
- Docker e Docker Compose (opcional, mas recomendado)

### Setup Local

```bash
# 1. Clone o repositório
git clone https://github.com/SrClauss/ominihub_axis
cd ominihub_axis

# 2. Configure as variáveis de ambiente
cp .env.example .env
# Edite .env com suas configurações

# 3. Inicie o banco de dados com PostGIS
docker-compose up -d

# 4. Compile e rode o projeto
cargo run
```

As migrations são executadas automaticamente na inicialização.

---

## Configuração

### Variáveis de Ambiente

```env
# Banco de dados principal
DATABASE_URL=postgres://postgres:postgres@localhost/axis_db

# Banco de dados de testes (opcional, usa DATABASE_URL como fallback)
TEST_DATABASE_URL=postgres://postgres:postgres@localhost/axis_test

# JWT — gera automaticamente se não informado
JWT_PRIVATE_KEY=
JWT_PUBLIC_KEY=

# Mercado Pago
MERCADOPAGO_ACCESS_TOKEN=your_token_here

# App
APP_BASE_URL=http://localhost:8080
RUST_LOG=info
```

> **Nota sobre JWT:** Se `JWT_PRIVATE_KEY` e `JWT_PUBLIC_KEY` não forem definidos, o AXIS gera automaticamente um par RSA 2048-bit na inicialização. Isso é adequado para desenvolvimento, mas em produção as chaves devem ser fixas para garantir que tokens permaneçam válidos entre reinicializações.

---

## API Reference

Veja [API.md](./API.md) para a documentação completa de todos os endpoints.

---

## Testes

Veja [TESTING.md](./TESTING.md) para o guia completo de execução e escrita de testes.

---

## Deploy

Veja [DEPLOY.md](./DEPLOY.md) para instruções de deploy em produção.
