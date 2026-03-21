# AXIS — API Reference

## Base URL

```
http://localhost:8080
```

## Autenticação

A maioria dos endpoints protegidos exige um token JWT no header:

```
Authorization: Bearer <access_token>
```

Os tokens são JWT RS256. O token de acesso expira em **1 hora**; o token de refresh expira em **30 dias**.

---

## Endpoints

### Health Check

#### `GET /health`

Verifica se o servidor está rodando.

**Resposta:**
```json
{
  "status": "ok",
  "service": "axis-core"
}
```

---

### Autenticação (`/auth`)

#### `POST /auth/register`

Registra um novo usuário.

**Body:**
```json
{
  "email": "user@example.com",
  "password": "senha123",
  "role": "driver",
  "home_hub_id": "uuid-opcional"
}
```

**Roles válidas:** `driver`, `passenger`, `franchisee`

**Resposta 201:**
```json
{
  "token": "eyJ...",
  "refresh_token": "eyJ...",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "role": "driver",
    "home_hub_id": null
  }
}
```

**Erros:**
- `400` — Role inválida
- `409` — Email já cadastrado

---

#### `POST /auth/login`

Autentica um usuário existente.

**Body:**
```json
{
  "email": "user@example.com",
  "password": "senha123"
}
```

**Resposta 200:**
```json
{
  "token": "eyJ...",
  "refresh_token": "eyJ...",
  "user": { ... }
}
```

**Erros:**
- `401` — Credenciais inválidas
- `403` — Conta inativa

---

#### `GET /auth/verify`

Verifica se um token de acesso é válido. Requer autenticação.

**Resposta 200:**
```json
{
  "valid": true,
  "user": { "id": "uuid", "email": "...", "role": "driver", "home_hub_id": null }
}
```

---

#### `POST /auth/refresh`

Gera um novo par de tokens usando o refresh token.

**Body:**
```json
{
  "refresh_token": "eyJ..."
}
```

**Resposta 200:** Mesmo formato do login.

---

#### `GET /auth/public-key`

Retorna a chave pública RSA usada para validar tokens JWT.

**Resposta 200:**
```json
{
  "public_key": "-----BEGIN PUBLIC KEY-----\n...",
  "algorithm": "RS256"
}
```

---

### Hubs (`/hubs`)

#### `POST /hubs/register`

Registra um novo hub no sistema.

**Body:**
```json
{
  "name": "Hub SP Centro",
  "slug": "sp-centro",
  "api_url": "https://hub-sp.example.com",
  "admin_email": "admin@hub-sp.com",
  "boundary": { "type": "Polygon", "coordinates": [[...]] }
}
```

---

#### `GET /hubs`

Lista todos os hubs cadastrados.

---

#### `GET /hubs/:id/status`

Retorna o status operacional de um hub.

**Resposta 200:**
```json
{
  "hub_id": "uuid",
  "operational_status": "active"
}
```

---

#### `PUT /hubs/:id/heartbeat`

Atualiza o timestamp de última atividade do hub.

---

#### `PUT /hubs/:id/boundary`

Atualiza o polígono de cobertura geográfica do hub.

---

#### `POST /hubs/:id/contains`

Verifica se uma localização está dentro do polígono do hub.

**Body:**
```json
{
  "latitude": -23.5505,
  "longitude": -46.6333
}
```

**Resposta 200:**
```json
{
  "contains": true
}
```

---

### Coverage (`/v1/coverage`)

#### `GET /v1/coverage/map`

Retorna o mapa de cobertura de todos os hubs ativos.

#### `GET /v1/coverage/version`

Retorna a versão atual do mapa de cobertura.

#### `POST /v1/coverage/validate`

Valida se uma localização está coberta por algum hub.

---

### Roaming (`/roaming`)

#### `POST /roaming/validate`

Valida se um motorista pode operar em um hub externo (roaming).

**Body:**
```json
{
  "driver_id": "uuid",
  "hub_id": "uuid"
}
```

---

### Admin — Pagamentos de Franquia (`/api/v1/admin`)

Requer role: `super_admin`, `hub_admin`, `admin`, `finance` ou `support`.

#### `POST /api/v1/admin/franchise-payments`

Cria um novo registro de cobrança de franquia.

**Body:**
```json
{
  "hub_id": "uuid",
  "due_date": "2026-04-01",
  "amount": 299.99,
  "notes": "Mensalidade abril/2026"
}
```

**Resposta 201:** Objeto `FranchisePayment`.

---

#### `GET /api/v1/admin/franchise-payments`

Lista todos os pagamentos. Suporta filtro `?hub_id=uuid`.

---

#### `PUT /api/v1/admin/franchise-payments/:id/mark-paid`

Marca um pagamento como pago manualmente.

**Body:**
```json
{
  "payment_method": "pix",
  "transaction_id": "TXN123",
  "notes": "Confirmado via comprovante"
}
```

---

#### `POST /api/v1/admin/franchise-payments/:id/adjustments`

Cria um ajuste (desconto, penalidade, crédito ou reembolso) para um pagamento.

**Body:**
```json
{
  "adjustment_type": "discount",
  "amount": 50.00,
  "reason": "Desconto por fidelidade"
}
```

**Tipos válidos:** `discount`, `penalty`, `credit`, `refund`

---

#### `GET /api/v1/admin/franchise-payments/:id/adjustments`

Lista os ajustes de um pagamento específico.

---

#### `GET /api/v1/admin/reports/payments`

Retorna relatório financeiro consolidado. Suporta filtros `?start_date=YYYY-MM-DD&end_date=YYYY-MM-DD`.

**Resposta 200:**
```json
{
  "total_franchises": 10,
  "active_franchises": 8,
  "overdue_franchises": 2,
  "total_revenue": 2399.92,
  "pending_revenue": 599.98,
  "by_status": {
    "active": 8,
    "grace": 1,
    "restricted": 1,
    "suspended": 0
  }
}
```

---

#### `GET /api/v1/admin/hubs/:id/status-history`

Retorna o histórico de mudanças de status operacional de um hub.

---

### Franqueado (`/api/v1/franchise`)

Requer role: `franchisee`. O hub é identificado automaticamente pelo email do usuário autenticado.

#### `GET /api/v1/franchise/payments`

Lista os pagamentos do hub do franqueado autenticado.

#### `GET /api/v1/franchise/payments/:id`

Retorna um pagamento específico (apenas do hub do franqueado).

#### `POST /api/v1/franchise/payments/:id/pay`

Gera um link de pagamento via Mercado Pago.

**Resposta 200:**
```json
{
  "payment_url": "https://www.mercadopago.com.br/checkout/..."
}
```

#### `GET /api/v1/franchise/dashboard`

Retorna o painel resumido do franqueado (status, pagamentos pendentes, alertas).

#### `GET /api/v1/franchise/notifications`

Lista as notificações do hub do franqueado.

#### `PUT /api/v1/franchise/notifications/:id/read`

Marca uma notificação como lida.

---

### Webhooks (`/api/v1/webhooks`)

#### `POST /api/v1/webhooks/payment-gateway`

Recebe notificações do gateway de pagamento (Mercado Pago). Não requer autenticação de usuário.

---

## Modelos de Dados

### FranchisePayment

```json
{
  "id": "uuid",
  "hub_id": "uuid",
  "due_date": "2026-04-01",
  "amount": 299.99,
  "status": "pending",
  "paid_at": null,
  "payment_method": null,
  "transaction_id": null,
  "gateway_payment_url": null,
  "notes": null,
  "created_at": "2026-03-21T00:00:00Z",
  "updated_at": "2026-03-21T00:00:00Z"
}
```

**Status possíveis:** `pending`, `paid`, `overdue`, `cancelled`

### User

```json
{
  "id": "uuid",
  "email": "user@example.com",
  "role": "driver",
  "home_hub_id": null
}
```

**Roles possíveis:** `driver`, `passenger`, `franchisee`, `admin`, `hub_admin`, `super_admin`, `finance`, `support`
