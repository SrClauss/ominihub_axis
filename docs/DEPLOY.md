# AXIS — Guia de Deploy

## Visão Geral

O AXIS pode ser implantado como um binário Rust nativo ou como um container Docker. Em produção, recomendamos Docker com PostgreSQL gerenciado.

---

## Pré-requisitos de Produção

- PostgreSQL 14+ com extensão PostGIS habilitada
- Variáveis de ambiente configuradas (ver seção Configuração)
- Chaves RSA fixas para JWT (geradas uma única vez)

---

## Deploy com Docker

### Build da imagem

```bash
docker build -t axis-core:latest .
```

### Rodando com Docker Compose

```yaml
# docker-compose.prod.yml
version: '3.8'
services:
  axis:
    image: axis-core:latest
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://axis:senha_segura@db/axis_db
      JWT_PRIVATE_KEY: |
        -----BEGIN PRIVATE KEY-----
        ...
        -----END PRIVATE KEY-----
      JWT_PUBLIC_KEY: |
        -----BEGIN PUBLIC KEY-----
        ...
        -----END PUBLIC KEY-----
      MERCADOPAGO_ACCESS_TOKEN: ${MERCADOPAGO_ACCESS_TOKEN}
      APP_BASE_URL: https://axis.seudominio.com
      RUST_LOG: info
    depends_on:
      - db

  db:
    image: postgis/postgis:16-3.4
    environment:
      POSTGRES_DB: axis_db
      POSTGRES_USER: axis
      POSTGRES_PASSWORD: senha_segura
    volumes:
      - pgdata:/var/lib/postgresql/data

volumes:
  pgdata:
```

```bash
docker-compose -f docker-compose.prod.yml up -d
```

---

## Deploy Manual (sem Docker)

### 1. Build de Release

```bash
cargo build --release
```

O binário estará em `target/release/axis-core`.

### 2. Gerar Chaves RSA

Execute uma vez para gerar e salvar as chaves:

```bash
openssl genrsa -out private_key.pem 2048
openssl rsa -in private_key.pem -pubout -out public_key.pem
```

Configure as variáveis de ambiente com o conteúdo dessas chaves.

### 3. Configurar Variáveis de Ambiente

```bash
export DATABASE_URL="postgres://user:pass@host/axis_db"
export JWT_PRIVATE_KEY="$(cat private_key.pem)"
export JWT_PUBLIC_KEY="$(cat public_key.pem)"
export MERCADOPAGO_ACCESS_TOKEN="seu_token_aqui"
export APP_BASE_URL="https://axis.seudominio.com"
export RUST_LOG="info"
```

### 4. Executar

```bash
./target/release/axis-core
```

As migrations são executadas automaticamente na inicialização.

---

## Configuração de Banco de Dados

### Habilitando PostGIS

```sql
CREATE EXTENSION IF NOT EXISTS postgis;
```

### Criando o banco de dados

```bash
createdb axis_db
psql axis_db -c "CREATE EXTENSION IF NOT EXISTS postgis;"
```

---

## Variáveis de Ambiente em Produção

| Variável                     | Obrigatória | Descrição                                         |
|------------------------------|-------------|---------------------------------------------------|
| `DATABASE_URL`               | ✅ Sim      | URL de conexão PostgreSQL                         |
| `JWT_PRIVATE_KEY`            | Recomendada | Chave privada RSA em PEM. Se omitida, gera nova a cada inicialização. |
| `JWT_PUBLIC_KEY`             | Recomendada | Chave pública RSA em PEM. Necessária se `JWT_PRIVATE_KEY` for fornecida. |
| `MERCADOPAGO_ACCESS_TOKEN`   | Opcional    | Token do Mercado Pago. Se omitido, pagamentos via gateway estarão desabilitados. |
| `APP_BASE_URL`               | Opcional    | URL base do AXIS. Default: `http://localhost:8080` |
| `RUST_LOG`                   | Opcional    | Nível de log. Ex: `info`, `debug`, `warn`         |

> **Importante:** Em produção, sempre defina `JWT_PRIVATE_KEY` e `JWT_PUBLIC_KEY`. Se não definidas, novas chaves são geradas a cada reinicialização, invalidando todos os tokens emitidos anteriormente.

---

## Health Check e Monitoramento

O endpoint `/health` pode ser usado para health checks:

```bash
curl http://localhost:8080/health
# {"status":"ok","service":"axis-core"}
```

### Exemplo com load balancer (Nginx)

```nginx
upstream axis {
    server 127.0.0.1:8080;
}

server {
    listen 443 ssl;
    server_name axis.seudominio.com;

    location / {
        proxy_pass http://axis;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

---

## Backup

Faça backup regular do banco de dados PostgreSQL:

```bash
pg_dump $DATABASE_URL > backup_$(date +%Y%m%d).sql
```

---

## Atualizações

1. Build da nova versão: `cargo build --release`
2. Pare o serviço: `systemctl stop axis`
3. Substitua o binário
4. Inicie o serviço: `systemctl start axis`

As migrations novas são aplicadas automaticamente na inicialização.
