# Alimentify REST API Backend

A secure, high-performance REST API backend for the Alimentify nutrition tracking application, built with Rust, Axum, MongoDB, and Redis. This backend provides comprehensive nutrition data, AI-powered food scanning, and secure user authentication.

## 🚀 Tech Stack

- **Rust 1.70+** - Systems programming language for performance and safety
- **Axum 0.7** - Modern web framework built on Tokio
- **MongoDB** - NoSQL database for user data and profiles
- **Redis** - In-memory data store for session management
- **Google OAuth 2.0** - Secure authentication provider
- **Brevo SMTP** - Email verification service
- **JWT** - Token-based authentication
- **Gemini AI 2.0-flash** - AI-powered food image analysis
- **USDA FoodData Central API** - Comprehensive US food database
- **API Ninjas Nutrition API** - Global nutrition data (primary source)

## 📋 Prerequisites

- Rust 1.70+ and Cargo
- MongoDB instance (local or MongoDB Atlas)
- Redis instance (local or Upstash)
- Google OAuth credentials
- Brevo SMTP account
- Gemini API key
- FoodData Central API key
- API Ninjas API key

## 🏗️ Architecture

### Project Structure

```
alimentify_backend/
├── src/
│   ├── main.rs              # Application entry point
│   ├── config.rs            # Configuration management
│   ├── db.rs                # Database and AppState setup
│   ├── error.rs             # Custom error types
│   ├── models.rs            # Data models (User, etc.)
│   ├── routes.rs            # Route definitions
│   ├── handlers/            # Request handlers
│   │   ├── auth.rs          # Authentication endpoints
│   │   ├── nutrition.rs     # Food scanning with Gemini AI
│   │   ├── nutrition_info.rs # Ninja API nutrition search
│   │   ├── food_wiki.rs     # FoodData Central search
│   │   └── status.rs        # Health check
│   ├── middleware/          # Custom middleware
│   │   ├── auth.rs          # JWT authentication
│   │   ├── cors.rs          # CORS configuration
│   │   └── api_key.rs       # API key validation (production)
│   └── services/            # External service integrations
│       ├── auth_service.rs  # Google OAuth logic
│       ├── email_service.rs # Email sending via Brevo
│       ├── gemini_service.rs # Gemini AI integration
│       ├── ninja_service.rs  # API Ninjas integration
│       └── fdc_service.rs    # FoodData Central integration
├── .env.local               # Environment variables (not in git)
├── Cargo.toml               # Rust dependencies
└── README.md                # This file
```

### Services Overview

1. **Authentication Service** (`auth_service.rs`)

   - Google OAuth 2.0 integration
   - JWT token generation and validation
   - Email verification workflow

2. **Email Service** (`email_service.rs`)

   - Send verification emails via Brevo SMTP
   - Customizable email templates

3. **Gemini Service** (`gemini_service.rs`)

   - Analyze food images using Google Gemini AI
   - Extract nutritional information from photos
   - Quick food identification

4. **Ninja Service** (`ninja_service.rs`)

   - Query API Ninjas for nutrition data
   - Parse nutrition facts (with premium feature detection)
   - Primary global nutrition source

5. **FDC Service** (`fdc_service.rs`)
   - Search USDA FoodData Central database
   - Get detailed food information by FDC ID
   - Batch food queries

## 🔧 Installation & Setup

### 1. Clone the Repository

```bash
git clone <repository-url>
cd alimentify_backend
```

### 2. Configure Environment Variables

Create a `.env.local` file in the project root:

```env
# SERVER CONFIGURATION
NODE_ENV=development
PORT=4000
HOST=0.0.0.0

# DATABASE - MONGODB
MONGODB_URI=mongodb+srv://<username>:<password>@<cluster>.mongodb.net/?retryWrites=true&w=majority
MONGODB_DATABASE=alimentify

# REDIS (Session Store)
REDIS_URL=redis://localhost:6379
# Or for Upstash:
# REDIS_URL=rediss://default:<password>@<host>.upstash.io:6379

# GOOGLE OAUTH
GOOGLE_CLIENT_ID=<your-google-client-id>
GOOGLE_CLIENT_SECRET=<your-google-client-secret>
GOOGLE_REDIRECT_URI=http://localhost:4000/api/auth/google/callback

# JWT CONFIGURATION
JWT_SECRET=<generate-random-secret-key>
JWT_EXPIRATION_HOURS=24

# BREVO EMAIL SERVICE (SMTP)
BREVO_SMTP_HOST=smtp-relay.brevo.com
BREVO_SMTP_PORT=587
BREVO_SMTP_USER=<your-brevo-user>
BREVO_SMTP_PASS=<your-brevo-password>
BREVO_FROM_EMAIL=noreply@yourdomain.com
BREVO_FROM_NAME=Alimentify

# SECURITY
# API keys (disabled in development, comma-separated in production)
API_KEYS=
# Require email verification (default: false in dev, true in prod)
REQUIRE_EMAIL_VERIFICATION=false

# Frontend origins for CORS
DEV_FRONTEND_ORIGIN=http://localhost:3000
PRODUCTION_FRONTEND_ORIGIN=https://yourdomain.com

# LOGGING
RUST_LOG=alimentify=debug,tower_http=debug,axum::rejection=trace

# API KEYS
GEMINI_API_KEY=<your-gemini-api-key>
FOOD_CENTRAL_API_KEY=<your-fdc-api-key>
NINJA_API_KEY=<your-ninja-api-key>
```

### 3. Install Dependencies & Build

```bash
cargo build --release
```

### 4. Run the Server

**Development mode:**

```bash
cargo run
```

**Production mode:**

```bash
cargo run --release
```

The server will start on `http://localhost:4000` (or the port specified in `.env.local`).

## 📚 API Documentation

### Base URL

```
http://localhost:4000/api
```

### Authentication

All protected endpoints require a JWT token in the Authorization header:

```
Authorization: Bearer <token>
```

---

### 🔓 Public Endpoints

#### Health Check

```http
GET /status
```

**Response:**

```json
{
  "status": "ok",
  "message": "Alimentify API is running",
  "timestamp": "2025-10-23T12:00:00Z"
}
```

#### Get Google OAuth URL

```http
GET /api/auth/google
```

**Response:**

```json
{
  "url": "https://accounts.google.com/o/oauth2/v2/auth?..."
}
```

#### Google OAuth Callback

```http
GET /api/auth/google/callback?code=<auth_code>
```

**Response:**

```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user": {
    "id": "507f1f77bcf86cd799439011",
    "name": "John Doe",
    "username": "johndoe",
    "gmail": "john@example.com",
    "profile_image": "https://...",
    "email_verification_status": true,
    "created_at": "2025-01-15T10:30:00Z"
  }
}
```

#### Verify Email

```http
GET /api/auth/verify-email?token=<verification_token>
```

**Response:**

```json
{
  "message": "Email verified successfully"
}
```

---

### 🔒 Protected Endpoints

#### Get Current User

```http
GET /api/auth/me
Authorization: Bearer <token>
```

**Response:**

```json
{
  "id": "507f1f77bcf86cd799439011",
  "name": "John Doe",
  "username": "johndoe",
  "gmail": "john@example.com",
  "profile_image": "https://...",
  "email_verification_status": true,
  "created_at": "2025-01-15T10:30:00Z"
}
```

#### Logout

```http
POST /api/auth/logout
Authorization: Bearer <token>
```

**Response:**

```json
{
  "message": "Logged out successfully"
}
```

---

### 🍎 Nutrition Endpoints

#### Analyze Food Image (Gemini AI)

```http
POST /api/nutrition/analyze
Authorization: Bearer <token>
Content-Type: multipart/form-data

image: <file>
```

**Response:**

```json
{
  "analysis": "This appears to be a bowl of oatmeal with banana slices...",
  "nutritional_info": {
    "calories": 350,
    "protein": 12,
    "carbohydrates": 58,
    "fat": 8,
    "fiber": 10
  }
}
```

#### Quick Food Check (Gemini AI)

```http
POST /api/nutrition/quick-check
Authorization: Bearer <token>
Content-Type: multipart/form-data

image: <file>
```

**Response:**

```json
{
  "food_name": "Banana",
  "confidence": "high",
  "quick_facts": "Rich in potassium, good source of vitamin B6"
}
```

---

### 🥗 Nutrition Info (API Ninjas - Primary)

#### Search Nutrition Info

```http
GET /api/nutrition-info?query=<food_query>
Authorization: Bearer <token>
```

**Example:**

```http
GET /api/nutrition-info?query=100g chicken breast
```

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "name": "chicken breast",
      "calories": 165.0,
      "serving_size_g": 100.0,
      "fat_total_g": 3.6,
      "fat_saturated_g": 1.0,
      "protein_g": 31.0,
      "sodium_mg": 74,
      "potassium_mg": 256,
      "cholesterol_mg": 85,
      "carbohydrates_total_g": 0.0,
      "fiber_g": 0.0,
      "sugar_g": 0.0
    }
  ],
  "message": null
}
```

**Note:** Free tier shows "Only available for premium subscribers" for some fields (calories, serving_size_g, protein_g). The API will return 0.0 for these fields.

---

### 📖 Food Wiki (USDA FoodData Central - US Focus)

#### Search Foods

```http
GET /api/food-wiki/search?query=<search_term>&pageNumber=1&pageSize=20
Authorization: Bearer <token>
```

**Query Parameters:**

- `query` (required): Search term
- `pageNumber` (optional): Page number (default: 1)
- `pageSize` (optional): Results per page (default: 20, max: 200)
- `dataType` (optional): Filter by data type (e.g., "Branded,Foundation")

**Example:**

```http
GET /api/food-wiki/search?query=apple&pageNumber=1&pageSize=10
```

**Response:**

```json
{
  "success": true,
  "data": {
    "totalHits": 1245,
    "currentPage": 1,
    "totalPages": 125,
    "foods": [
      {
        "fdcId": 171688,
        "description": "Apple, raw",
        "dataType": "Foundation",
        "gtinUpc": null,
        "brandOwner": null,
        "brandName": null,
        "ingredients": null,
        "foodNutrients": [...]
      }
    ]
  }
}
```

#### Get Food Details

```http
GET /api/food-wiki/:fdc_id
Authorization: Bearer <token>
```

**Example:**

```http
GET /api/food-wiki/171688
```

**Response:**

```json
{
  "success": true,
  "data": {
    "fdcId": 171688,
    "description": "Apple, raw",
    "dataType": "Foundation",
    "foodClass": "FinalFood",
    "foodCategory": {
      "id": 9,
      "code": "0900",
      "description": "Fruits and Fruit Juices"
    },
    "foodNutrients": [
      {
        "id": 1234,
        "amount": 52.0,
        "nutrient": {
          "id": 1008,
          "number": "208",
          "name": "Energy",
          "unitName": "kcal"
        }
      }
    ],
    "foodPortions": [
      {
        "id": 1,
        "amount": 1.0,
        "modifier": "medium",
        "gramWeight": 182.0
      }
    ]
  }
}
```

#### Get Multiple Foods

```http
POST /api/food-wiki/foods
Authorization: Bearer <token>
Content-Type: application/json

{
  "fdcIds": [171688, 171689, 171690]
}
```

**Response:**

```json
{
  "success": true,
  "data": [
    {
      "fdcId": 171688,
      "description": "Apple, raw",
      ...
    },
    {
      "fdcId": 171689,
      "description": "Banana, raw",
      ...
    }
  ]
}
```

---

## 🔐 Security Features

### Middleware

1. **CORS Middleware** (`middleware/cors.rs`)

   - Enabled in development for `http://localhost:3000`
   - Configurable for production origins
   - Allows credentials, common headers

2. **API Key Middleware** (`middleware/api_key.rs`)

   - Disabled in development (`NODE_ENV=development`)
   - Required in production
   - Validates `X-API-Key` header

3. **Auth Middleware** (`middleware/auth.rs`)
   - Validates JWT tokens
   - Extracts user information
   - Protects all `/api/auth/me`, `/api/nutrition/*`, `/api/food-wiki/*`, etc.

### Authentication Flow

1. User clicks "Login with Google" → Frontend redirects to `/api/auth/google`
2. Backend generates Google OAuth URL
3. User authorizes on Google
4. Google redirects to `/api/auth/google/callback?code=...`
5. Backend exchanges code for user info
6. Backend creates/updates user in MongoDB
7. Backend generates JWT token
8. Frontend stores token in localStorage
9. Frontend includes token in `Authorization: Bearer <token>` header for protected routes

### Email Verification

- Optional in development (`REQUIRE_EMAIL_VERIFICATION=false`)
- Sends verification email via Brevo SMTP
- Verification link: `/api/auth/verify-email?token=<token>`
- Token expires after 24 hours

---

## 🧪 Testing

### Manual Testing with cURL

**Health Check:**

```bash
curl http://localhost:4000/status
```

**Get Nutrition Info:**

```bash
curl -H "Authorization: Bearer YOUR_TOKEN" \
  "http://localhost:4000/api/nutrition-info?query=100g%20apple"
```

**Search Food Wiki:**

```bash
curl -H "Authorization: Bearer YOUR_TOKEN" \
  "http://localhost:4000/api/food-wiki/search?query=chicken&pageSize=5"
```

**Analyze Food Image:**

```bash
curl -X POST \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -F "image=@/path/to/food.jpg" \
  http://localhost:4000/api/nutrition/analyze
```

---

## 🚀 Deployment

### Environment-Specific Configuration

The backend automatically detects the environment from `NODE_ENV`:

- **Development** (`NODE_ENV=development`):

  - CORS enabled for localhost
  - API key validation disabled
  - Email verification optional
  - Detailed logging

- **Production** (`NODE_ENV=production`):
  - CORS restricted to `PRODUCTION_FRONTEND_ORIGIN`
  - API key validation enabled
  - Email verification enforced
  - Error details hidden

### Deployment Options

1. **VPS/Dedicated Server**

   ```bash
   cargo build --release
   ./target/release/alimentify
   ```

2. **Docker**

   ```dockerfile
   FROM rust:1.70 as builder
   WORKDIR /app
   COPY . .
   RUN cargo build --release

   FROM debian:bookworm-slim
   COPY --from=builder /app/target/release/alimentify /usr/local/bin/
   CMD ["alimentify"]
   ```

3. **Cloud Platforms**
   - Railway, Fly.io, Render: Connect repo and configure build command
   - AWS/GCP/Azure: Deploy as container or binary

### Production Checklist

- [ ] Set `NODE_ENV=production`
- [ ] Configure `PRODUCTION_FRONTEND_ORIGIN`
- [ ] Add production API keys to `API_KEYS`
- [ ] Set secure `JWT_SECRET` (32+ random characters)
- [ ] Enable `REQUIRE_EMAIL_VERIFICATION=true`
- [ ] Use production MongoDB and Redis instances
- [ ] Configure firewall rules (only allow 4000 from frontend)
- [ ] Set up SSL/TLS (use reverse proxy like Nginx)
- [ ] Monitor logs (`RUST_LOG=alimentify=info`)

---

## 📊 Error Handling

All errors return JSON responses:

```json
{
  "error": "Error message here"
}
```

**HTTP Status Codes:**

- `200 OK` - Success
- `400 Bad Request` - Invalid input
- `401 Unauthorized` - Missing/invalid token
- `403 Forbidden` - Insufficient permissions
- `404 Not Found` - Resource not found
- `422 Unprocessable Entity` - Validation error
- `500 Internal Server Error` - Server error

---

## 🛠️ Development Tips

### Enable Debug Logging

```env
RUST_LOG=alimentify=debug,tower_http=debug,axum::rejection=trace
```

### Watch Mode (Auto-reload)

Install `cargo-watch`:

```bash
cargo install cargo-watch
cargo watch -x run
```

### Format Code

```bash
cargo fmt
```

### Lint Code

```bash
cargo clippy
```

---

## 📄 License

MIT License - feel free to use this project as you wish.

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## 📞 Support

For issues or questions:

- Open an issue in the repository
- Check existing documentation
- Review error logs (`RUST_LOG=debug`)

---

## 🔗 Related Resources

- [Axum Documentation](https://docs.rs/axum)
- [MongoDB Rust Driver](https://docs.rs/mongodb)
- [Google OAuth 2.0](https://developers.google.com/identity/protocols/oauth2)
- [Gemini API](https://ai.google.dev/docs)
- [USDA FoodData Central](https://fdc.nal.usda.gov/api-guide.html)
- [API Ninjas](https://www.api-ninjas.com/api/nutrition)
