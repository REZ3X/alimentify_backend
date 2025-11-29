use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport,
    AsyncTransport,
    Message,
    Tokio1Executor,
};

use crate::{ config::Config, error::Result };

pub async fn send_verification_email(
    config: &Config,
    to_email: &str,
    to_name: &str,
    token: &str
) -> Result<()> {
    let verification_url = format!(
        "{}/auth/verify-email?token={}",
        config.security.allowed_origins.first().unwrap_or(&"http://localhost:3000".to_string()),
        token
    );

    let email_body = format!(
        r#"
        <html>
            <body style="font-family: Arial, sans-serif; padding: 20px;">
                <h2>Welcome to Alimentify!</h2>
                <p>Hello {},</p>
                <p>Thank you for registering with Alimentify. Please verify your email address by clicking the button below:</p>
                <p style="margin: 30px 0;">
                    <a href="{}" style="background-color: #4CAF50; color: white; padding: 14px 20px; text-decoration: none; border-radius: 4px;">
                        Verify Email
                    </a>
                </p>
                <p>Or copy and paste this link into your browser:</p>
                <p><a href="{}">{}</a></p>
                <p>This link will expire in 24 hours.</p>
                <p>If you didn't create an account, please ignore this email.</p>
                <br>
                <p>Best regards,<br>The Alimentify Team</p>
            </body>
        </html>
        "#,
        to_name,
        verification_url,
        verification_url,
        verification_url
    );

    let email = Message::builder()
        .from(format!("{} <{}>", config.brevo.from_name, config.brevo.from_email).parse().unwrap())
        .to(format!("{} <{}>", to_name, to_email).parse().unwrap())
        .subject("Verify your Alimentify account")
        .header(ContentType::TEXT_HTML)
        .body(email_body)
        .unwrap();

    let creds = Credentials::new(config.brevo.smtp_user.clone(), config.brevo.smtp_pass.clone());

    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>
        ::starttls_relay(&config.brevo.smtp_host)
        .unwrap()
        .port(config.brevo.smtp_port)
        .credentials(creds)
        .build();

    mailer.send(email).await.map_err(|e| {
        tracing::error!("Failed to send email: {}", e);
        crate::error::AppError::InternalError(anyhow::anyhow!("Failed to send email"))
    })?;

    tracing::info!("Verification email sent to {}", to_email);

    Ok(())
}
