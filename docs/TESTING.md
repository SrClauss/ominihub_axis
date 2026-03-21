# AXIS — Guia de Testes

## Visão Geral

A suite de testes do AXIS está organizada em três camadas:

| Camada       | Localização                 | O que testa                              |
|--------------|-----------------------------|------------------------------------------|
| Unitários    | `tests/unit/`               | Services e funções isoladas              |
| Integração   | `tests/integration/`        | Endpoints HTTP com banco de dados real   |
| E2E          | `tests/hub_contains.rs`     | Operações geoespaciais com PostGIS       |

---

## Pré-requisitos

Para rodar os testes que usam banco de dados, você precisa de:

1. PostgreSQL 14+ com extensão PostGIS rodando
2. A variável de ambiente `DATABASE_URL` configurada

```bash
# Exemplo usando Docker
docker-compose up -d

# Configurar a URL do banco
export DATABASE_URL="postgres://postgres:postgres@localhost/axis_test"
```

---

## Executando os Testes

### Todos os testes

```bash
cargo test
```

### Com output detalhado

```bash
cargo test -- --nocapture
```

### Apenas testes unitários

```bash
cargo test --test unit
```

### Apenas testes de integração

```bash
cargo test --test integration
```

### Apenas testes E2E geoespaciais

```bash
cargo test --test hub_contains
```

### Um teste específico por nome

```bash
cargo test test_compute_status
```

### Testes sequenciais (recomendado para evitar conflitos no banco)

```bash
cargo test -- --test-threads=1
```

---

## Estrutura dos Testes

```
tests/
├── hub_contains.rs            # Testes E2E de geolocalização (PostGIS)
├── common/
│   └── mod.rs                 # Helpers e fixtures compartilhados
├── unit/
│   ├── mod.rs                 # Entry point do crate de testes unitários
│   ├── auth_service_test.rs   # Testes do serviço de autenticação
│   ├── payment_service_test.rs    # Testes do PaymentService
│   ├── hub_status_service_test.rs # Testes do HubStatusService
│   └── block_service_test.rs  # Testes do BlockService
└── integration/
    ├── mod.rs                 # Entry point do crate de testes de integração
    ├── auth_endpoints_test.rs # Testes dos endpoints de autenticação
    └── payment_endpoints_test.rs  # Testes dos endpoints de pagamento
```

---

## Helpers Compartilhados (`tests/common/mod.rs`)

### `setup_test_db() -> PgPool`

Conecta ao banco de dados de teste e executa todas as migrations. Usa a variável `DATABASE_URL`.

### `cleanup_test_db(pool: &PgPool)`

Trunca todas as tabelas de dados com `CASCADE`. Use ao final de cada teste para isolar o estado.

### `create_test_user(pool, email, role) -> User`

Cria um usuário com senha padrão `"test123"`.

### `create_test_hub(pool, name, slug) -> Uuid`

Cria um hub com polígono de teste pré-definido.

### `create_test_hub_for_franchisee(pool, name, slug, admin_email) -> Uuid`

Cria um hub vinculado ao email de um franqueado (necessário para testes de endpoints de franqueado).

### `generate_test_jwt(user_id, role, private_key) -> String`

Gera um token de acesso JWT assinado com a chave privada fornecida.

### `test_rsa_keys() -> (String, String)`

Gera um par de chaves RSA para uso nos testes de integração.

---

## Padrões de Teste

### Testes Unitários (sem banco de dados)

```rust
#[test]
fn test_compute_status() {
    assert_eq!(HubStatusService::compute_status(0), "active");
    assert_eq!(HubStatusService::compute_status(15), "suspended");
}
```

### Testes com Banco de Dados

```rust
#[tokio::test]
async fn test_create_payment() {
    let pool = setup_test_db().await;
    let hub_id = create_test_hub(&pool, "Test Hub", "test-hub").await;

    let service = PaymentService::new(pool.clone());
    let payment = service.create_payment(/* ... */).await.unwrap();

    assert_eq!(payment.status, "pending");

    cleanup_test_db(&pool).await; // sempre limpar ao final
}
```

### Testes de Integração (endpoints HTTP)

```rust
#[tokio::test]
async fn test_register_endpoint() {
    let pool = setup_test_db().await;
    let (priv_key, pub_key) = test_rsa_keys();
    let app = axis_core::build_test_router(pool.clone(), priv_key, pub_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "email": "...", ... }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    cleanup_test_db(&pool).await;
}
```

A função `build_test_router` cria o roteador com o pool e o par de chaves JWT especificados. Isso garante que os tokens gerados nos testes sejam válidos para o app sob teste.

---

## Boas Práticas

1. **Sempre limpe o banco ao final do teste** com `cleanup_test_db`.
2. **Use emails únicos por teste** para evitar conflitos de unique constraint (ex: sufixos distintos em cada função).
3. **Prefira `--test-threads=1`** ao rodar testes que modificam o banco em paralelo.
4. **Testes unitários puros** (sem DB) devem usar `#[test]`, não `#[tokio::test]`.
5. **Use `build_test_router`** nos testes de integração para garantir que o JWT key pair seja conhecido.

---

## Cobertura de Código

Para gerar um relatório de cobertura HTML:

```bash
# Instalar cargo-tarpaulin
cargo install cargo-tarpaulin

# Gerar relatório (exclui código de migração)
cargo tarpaulin --out Html --exclude-files "migrations/*"
```

O relatório será gerado em `tarpaulin-report.html`.
