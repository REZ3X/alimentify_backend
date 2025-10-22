# Alimentify REST API Backend

A secure, high-performance REST API backend for the Alimentify web application, built with Rust, Axum, MongoDB, and Redis.

## 🚀 Tech Stack

- **Rust** - Systems programming language for performance and safety
- **Axum** - Ergonomic web framework built on Tokio
- **MongoDB** - NoSQL database for user data
- **Redis** - In-memory data store for session management
- **Google OAuth 2.0** - Authentication
- **Brevo (SMTP)** - Email verification service
- **JWT** - Secure token-based authentication

## 🔒 Security Features

### Environment-Based Security

- **Development Mode**:
  - ✅ CORS disabled (allows all origins)
  - ✅ API Key authentication disabled
  - ✅ Detailed logging
- **Production Mode**:
  - 🔒 CORS enabled with strict origin checking
  - 🔒 API Key authentication required
  - 🔒 Optimized logging

### Security Layers

1. **CORS Protection** - Cross-Origin Resource Sharing restrictions
2. **API Key Validation** - Secure API access with key validation
3. **JWT Authentication** - Token-based user authentication
4. **Email Verification** - User email validation via Brevo
5. **Session Management** - Redis-based session storage

## 📋 Prerequisites

Before you begin, ensure you have installed:

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [MongoDB](https://www.mongodb.com/try/download/community) (v4.4+)
- [Redis](https://redis.io/download) (v6.0+)
- [Trunk](https://trunkrs.dev/) (optional, for frontend integration)

### Windows Setup

```powershell
# Install Rust
# Download from: https://www.rust-lang.org/tools/install

# Install MongoDB
# Download from: https://www.mongodb.com/try/download/community

# Install Redis (using Chocolatey)
choco install redis-64

# Or download from: https://github.com/microsoftarchive/redis/releases
```

## 🛠️ Setup Instructions

### 1. Clone and Navigate

```powershell
cd "d:\Next Project\Apps\techcomtek\alimentify_backend"
```

### 2. Configure Environment

```powershell
# Copy example env file
Copy-Item .env.example .env.local
```

Edit `.env.local` and fill in your credentials:

```env
# Google OAuth (Get from Google Cloud Console)
GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your-client-secret

# MongoDB (local or cloud)
MONGODB_URI=mongodb://localhost:27017

# Brevo Email (Get from Brevo dashboard)
BREVO_SMTP_USER=your-brevo-user
BREVO_SMTP_PASS=your-brevo-password
BREVO_FROM_EMAIL=noreply@yourdomain.com
```

### 3. Start Required Services

```powershell
# Start MongoDB
mongod --dbpath "C:\data\db"

# Start Redis
redis-server
```

### 4. Install Dependencies and Build

```powershell
cargo build
```

### 5. Run the Development Server

```powershell
cargo run
```

The API server will start on `http://localhost:4000`

### 6. Run in Production Mode

```powershell
# Set environment to production
$env:NODE_ENV="production"

# Build and run in release mode
cargo run --release
```

## 📚 API Endpoints

### Public Endpoints (No Authentication)

#### Health Check

```
GET /status
```

Returns server status and health information.

**Response:**

```json
{
  "status": "healthy",
  "service": "Alimentify API",
  "version": "0.1.0",
  "timestamp": "2025-10-21T10:30:00Z",
  "environment": "development"
}
```

### Authentication Endpoints

#### Get Google OAuth URL

```
GET /api/auth/google
```

Returns the Google OAuth authorization URL.

**Response:**

```json
{
  "auth_url": "https://accounts.google.com/o/oauth2/v2/auth?..."
}
```

#### Google OAuth Callback

```
GET /api/auth/google/callback?code={code}
```

Handles Google OAuth callback and returns JWT token.

**Response:**

```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user": {
    "id": "507f1f77bcf86cd799439011",
    "google_id": "1234567890",
    "username": "johndoe",
    "name": "John Doe",
    "gmail": "john@example.com",
    "profile_image": "https://...",
    "email_verification_status": false,
    "email_verified_at": null,
    "created_at": "2025-10-21T10:00:00Z",
    "updated_at": "2025-10-21T10:00:00Z"
  }
}
```

#### Verify Email

```
GET /api/auth/verify-email?token={token}
```

Verifies user email with the token sent via email.

**Response:**

```json
{
  "message": "Email verified successfully"
}
```

### Protected Endpoints (Require JWT)

All protected endpoints require the `Authorization` header:

```
Authorization: Bearer {your-jwt-token}
```

#### Get Current User

```
GET /api/auth/me
```

Returns the authenticated user's information.

**Response:**

```json
{
  "id": "507f1f77bcf86cd799439011",
  "google_id": "1234567890",
  "username": "johndoe",
  "name": "John Doe",
  "gmail": "john@example.com",
  "profile_image": "https://...",
  "email_verification_status": true,
  "email_verified_at": "2025-10-21T10:30:00Z",
  "created_at": "2025-10-21T10:00:00Z",
  "updated_at": "2025-10-21T10:30:00Z"
}
```

#### Logout

```
POST /api/auth/logout
```

Invalidates the user's session.

**Response:**

```
204 No Content
```

## 📝 Example API Requests

### Using PowerShell

#### Check Server Status

```powershell
Invoke-RestMethod -Uri "http://localhost:4000/status" -Method Get
```

#### Get Google Auth URL

```powershell
$response = Invoke-RestMethod -Uri "http://localhost:4000/api/auth/google" -Method Get
$response.auth_url
```

#### Get Current User (with JWT)

```powershell
$headers = @{
    "Authorization" = "Bearer your-jwt-token-here"
}
Invoke-RestMethod -Uri "http://localhost:4000/api/auth/me" -Method Get -Headers $headers
```

#### Logout

```powershell
$headers = @{
    "Authorization" = "Bearer your-jwt-token-here"
}
Invoke-RestMethod -Uri "http://localhost:4000/api/auth/logout" -Method Post -Headers $headers
```

## 🏗️ Project Structure

```
alimentify_backend/
├── src/
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Environment configuration
│   ├── db.rs                # Database connections (MongoDB, Redis)
│   ├── routes.rs            # API route definitions
│   ├── models.rs            # Data models and DTOs
│   ├── error.rs             # Error handling
│   ├── handlers/            # Request handlers
│   │   ├── mod.rs
│   │   ├── status.rs        # Health check handler
│   │   └── auth.rs          # Authentication handlers
│   ├── middleware/          # Custom middleware
│   │   ├── mod.rs
│   │   ├── api_key.rs       # API key validation
│   │   ├── auth.rs          # JWT authentication
│   │   └── cors.rs          # CORS configuration
│   └── services/            # Business logic
│       ├── mod.rs
│       ├── auth_service.rs  # Auth logic (Google OAuth, JWT)
│       └── email_service.rs # Email sending (Brevo)
├── Cargo.toml               # Project dependencies
├── .env.example             # Environment variables template
├── .env.local               # Local environment variables (not committed)
├── .gitignore              # Git ignore rules
└── README.md               # This file
```

## 🗄️ Database Schema

### User Collection (MongoDB)

```javascript
{
  "_id": ObjectId,
  "google_id": String,
  "profile_image": String (optional),
  "username": String,
  "name": String,
  "gmail": String,
  "email_verification_status": Boolean,
  "email_verification_token": String (optional),
  "email_verified_at": DateTime (optional),
  "created_at": DateTime,
  "updated_at": DateTime
}
```

### Session (Redis)

```json
{
  "user_id": "String",
  "email": "String",
  "created_at": "DateTime",
  "expires_at": "DateTime"
}
```

**Key format:** `session:{user_id}`  
**TTL:** 24 hours

## 🔧 Development

### Run with Auto-Reload

```powershell
cargo install cargo-watch
cargo watch -x run
```

### Run Tests

```powershell
cargo test
```

### Format Code

```powershell
cargo fmt
```

### Lint Code

```powershell
cargo clippy
```

## 🌐 Google OAuth Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select existing
3. Enable Google+ API
4. Go to "Credentials" → "Create Credentials" → "OAuth 2.0 Client ID"
5. Set authorized redirect URI: `http://localhost:4000/api/auth/google/callback`
6. Copy Client ID and Client Secret to `.env.local`

## 📧 Brevo Email Setup

1. Sign up at [Brevo](https://www.brevo.com/)
2. Go to "SMTP & API" section
3. Generate SMTP credentials
4. Copy credentials to `.env.local`

## 📦 Building for Production

```powershell
# Build optimized binary
cargo build --release

# The binary will be at:
# target\release\alimentify.exe
```

## 🚀 Deployment

### Deploy to VPS

```powershell
# Build for release
cargo build --release

# Copy binary and .env to server
scp target/release/alimentify user@server:/opt/alimentify/
scp .env.production user@server:/opt/alimentify/.env

# Run on server
./alimentify
```

### Docker Deployment (Optional)

Create a `Dockerfile`:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/alimentify /usr/local/bin/
CMD ["alimentify"]
```

## 🔐 Security Best Practices

1. **Never commit `.env.local`** - Contains sensitive credentials
2. **Use strong JWT secrets** - Generate random 32+ character strings
3. **Enable CORS in production** - Whitelist only your frontend domains
4. **Use HTTPS in production** - Never use HTTP for authentication
5. **Rotate API keys regularly** - Update keys periodically
6. **Monitor logs** - Watch for suspicious activity
7. **Keep dependencies updated** - Run `cargo update` regularly

## 📄 License

MIT License - feel free to use this project as you wish.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📞 Support

For issues or questions, please open an issue in the repository.

---

Built with ❤️ using Rust and Axum

## 🚀 Tech Stack

- **Rust** - Systems programming language for performance and safety
- **Axum** - Ergonomic web framework built on Tokio
- **Tokio** - Async runtime for Rust
- **Trunk** - Build and bundler for WebAssembly (for frontend integration)
- **Serde** - Serialization/deserialization
- **Tower** - Middleware and service abstractions

## 📋 Prerequisites

Before you begin, ensure you have installed:

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Cargo](https://doc.rust-lang.org/cargo/) (comes with Rust)
- [Trunk](https://trunkrs.dev/) (optional, for frontend):
  ```powershell
  cargo install trunk
  ```

## 🛠️ Setup Instructions

### 1. Clone and Navigate

```powershell
cd "d:\Next Project\Apps\techcomtek"
```

### 2. Configure Environment

Copy the example environment file and customize it:

```powershell
Copy-Item .env.example .env
```

Edit `.env` file with your preferred settings (port, database URL, etc.)

### 3. Install Dependencies

```powershell
cargo build
```

### 4. Run the Development Server

```powershell
cargo run
```

The API server will start on `http://localhost:3000` (or your configured PORT).

### 5. Run in Release Mode (Production)

```powershell
cargo run --release
```

## 📚 API Endpoints

### Health Check

- `GET /` - Root health check
- `GET /api/health` - Detailed health status

### Items/Products

- `GET /api/v1/items` - List all items
- `POST /api/v1/items` - Create a new item
- `GET /api/v1/items/:id` - Get item by ID
- `PUT /api/v1/items/:id` - Update item by ID
- `DELETE /api/v1/items/:id` - Delete item by ID

### Users

- `GET /api/v1/users` - List all users
- `POST /api/v1/users` - Create a new user
- `GET /api/v1/users/:id` - Get user by ID

## 📝 Example API Requests

### Create an Item

```powershell
curl -X POST http://localhost:3000/api/v1/items `
  -H "Content-Type: application/json" `
  -d '{
    "name": "Apple",
    "description": "Fresh red apple",
    "price": 2.50,
    "quantity": 100,
    "category": "Fruits"
  }'
```

### Get All Items

```powershell
curl http://localhost:3000/api/v1/items
```

### Create a User

```powershell
curl -X POST http://localhost:3000/api/v1/users `
  -H "Content-Type: application/json" `
  -d '{
    "username": "johndoe",
    "email": "john@example.com"
  }'
```

## 🏗️ Project Structure

```
alimentify/
├── src/
│   ├── main.rs           # Application entry point
│   ├── routes.rs         # API route definitions
│   ├── models.rs         # Data models and DTOs
│   ├── error.rs          # Error handling
│   └── handlers/         # Request handlers
│       ├── mod.rs
│       ├── health.rs
│       ├── items.rs
│       └── users.rs
├── Cargo.toml            # Project dependencies
├── .env.example          # Environment variables template
└── README.md             # This file
```

## 🔧 Development

### Run with Auto-Reload

Install cargo-watch:

```powershell
cargo install cargo-watch
```

Then run:

```powershell
cargo watch -x run
```

### Run Tests

```powershell
cargo test
```

### Format Code

```powershell
cargo fmt
```

### Lint Code

```powershell
cargo clippy
```

## 🗄️ Database Integration

The current implementation uses in-memory storage. To integrate a real database (PostgreSQL):

1. Uncomment the `sqlx` dependency in `Cargo.toml`
2. Set up your database and update `DATABASE_URL` in `.env`
3. Create migrations and replace the in-memory storage in handlers

## 🌐 CORS Configuration

CORS is currently configured to allow all origins. For production, update the CORS layer in `main.rs`:

```rust
let cors = CorsLayer::new()
    .allow_origin("https://yourdomain.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE]);
```

## 📦 Building for Production

```powershell
cargo build --release
```

The optimized binary will be in `target/release/alimentify.exe`

## 🚀 Deployment

The compiled binary can be deployed to:

- VPS or dedicated server
- Docker container
- Cloud platforms (AWS, Azure, GCP)
- Platform-as-a-Service (Heroku, Railway, Fly.io)

## 📄 License

MIT License - feel free to use this project as you wish.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📞 Support

For issues or questions, please open an issue in the repository.
