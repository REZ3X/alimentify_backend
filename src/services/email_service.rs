use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport,
    AsyncTransport,
    Message,
    Tokio1Executor,
};

use crate::{ config::Config, error::Result, models::{User, MealReport} };

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

pub struct EmailService {
    smtp_host: String,
    smtp_port: u16,
    smtp_username: String,
    smtp_password: String,
    from_email: String,
    from_name: String,
}

impl EmailService {
    pub fn new(
        smtp_host: String,
        smtp_port: u16,
        smtp_username: String,
        smtp_password: String,
        from_email: String,
        from_name: String,
    ) -> Self {
        Self {
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            from_email,
            from_name,
        }
    }

    pub async fn send_report_email(&self, user: &User, report: &MealReport) -> Result<()> {
        let report_period = match report.report_type {
            crate::models::ReportPeriod::Daily => "Daily",
            crate::models::ReportPeriod::Weekly => "Weekly",
            crate::models::ReportPeriod::Monthly => "Monthly",
            crate::models::ReportPeriod::Yearly => "Yearly",
        };

        let goal_status_emoji = if report.goal_achieved { "🎉" } else { "📊" };
        let goal_status_text = if report.goal_achieved {
            "<span style='color: #4CAF50; font-weight: bold;'>✅ ACHIEVED</span>"
        } else {
            "<span style='color: #FF9800; font-weight: bold;'>⏳ IN PROGRESS</span>"
        };

        let weight_section = if let (Some(start), Some(end), Some(change), Some(target)) = 
            (report.starting_weight, report.ending_weight, report.weight_change, report.target_weight) {
            format!(
                r#"
                <div style="background-color: #E3F2FD; padding: 15px; border-radius: 8px; margin: 15px 0;">
                    <h3 style="color: #1976D2; margin-top: 0;">💪 Weight Progress</h3>
                    <p><strong>Starting Weight:</strong> {:.1} kg</p>
                    <p><strong>Current Weight:</strong> {:.1} kg</p>
                    <p><strong>Change:</strong> {:+.1} kg</p>
                    <p><strong>Target Weight:</strong> {:.1} kg</p>
                    <p><strong>Goal Status:</strong> {}</p>
                </div>
                "#,
                start, end, change, target,
                if report.weight_goal_achieved.unwrap_or(false) {
                    "✅ Reached"
                } else {
                    "⏳ In Progress"
                }
            )
        } else {
            String::new()
        };

        let best_day_section = if let (Some(date), Some(compliance)) = 
            (&report.best_day_date, report.best_day_compliance) {
            format!(
                r#"
                <p><strong>🏆 Best Day:</strong> {} ({:.1}% compliance)</p>
                "#,
                date, compliance
            )
        } else {
            String::new()
        };

        let email_body = format!(
            r#"
            <html>
                <head>
                    <style>
                        body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; padding: 20px; background-color: #f5f5f5; }}
                        .container {{ max-width: 600px; margin: 0 auto; background-color: white; border-radius: 12px; padding: 30px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
                        h1 {{ color: #2E7D32; }}
                        h2 {{ color: #388E3C; border-bottom: 2px solid #4CAF50; padding-bottom: 10px; }}
                        h3 {{ color: #4CAF50; }}
                        .metric {{ background-color: #F1F8E9; padding: 12px; border-radius: 6px; margin: 10px 0; }}
                        .metric-label {{ font-weight: bold; color: #558B2F; }}
                        .metric-value {{ font-size: 1.2em; color: #33691E; }}
                        .goal-badge {{ display: inline-block; padding: 8px 16px; border-radius: 20px; font-weight: bold; margin: 10px 0; }}
                        .achieved {{ background-color: #C8E6C9; color: #1B5E20; }}
                        .in-progress {{ background-color: #FFE0B2; color: #E65100; }}
                        .footer {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #E0E0E0; color: #757575; font-size: 0.9em; }}
                    </style>
                </head>
                <body>
                    <div class="container">
                        <h1>{} {} Nutrition Report</h1>
                        <p>Hello <strong>{}</strong>,</p>
                        <p>Here's your nutrition summary for <strong>{}</strong> to <strong>{}</strong></p>

                        <div style="background-color: {}; padding: 20px; border-radius: 8px; margin: 20px 0; text-align: center;">
                            <h2 style="margin: 0; color: white; border: none;">Goal Status: {}</h2>
                        </div>

                        <h2>📊 Summary Statistics</h2>
                        <div class="metric">
                            <span class="metric-label">Total Days in Period:</span>
                            <span class="metric-value">{}</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Days Logged:</span>
                            <span class="metric-value">{} ({:.1}%)</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Total Meals:</span>
                            <span class="metric-value">{}</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Logging Streak:</span>
                            <span class="metric-value">{} days 🔥</span>
                        </div>

                        <h2>🎯 Nutrition Averages</h2>
                        <div class="metric">
                            <span class="metric-label">Calories:</span>
                            <span class="metric-value">{:.0} kcal/day</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Protein:</span>
                            <span class="metric-value">{:.1}g/day</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Carbs:</span>
                            <span class="metric-value">{:.1}g/day</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Fat:</span>
                            <span class="metric-value">{:.1}g/day</span>
                        </div>

                        <h2>✅ Goal Compliance</h2>
                        <div class="metric">
                            <span class="metric-label">Calories Compliance:</span>
                            <span class="metric-value">{:.1}%</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Protein Compliance:</span>
                            <span class="metric-value">{:.1}%</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Carbs Compliance:</span>
                            <span class="metric-value">{:.1}%</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Fat Compliance:</span>
                            <span class="metric-value">{:.1}%</span>
                        </div>
                        <div class="metric">
                            <span class="metric-label">Days On Target:</span>
                            <span class="metric-value">{}/{}</span>
                        </div>

                        {}

                        {}

                        <h2>💡 Keep Going!</h2>
                        <p>{}</p>

                        <div class="footer">
                            <p>This is an automated report from Alimentify. View more details in your dashboard.</p>
                            <p>Best regards,<br><strong>The Alimentify Team</strong></p>
                        </div>
                    </div>
                </body>
            </html>
            "#,
            goal_status_emoji,
            report_period,
            user.name,
            report.start_date,
            report.end_date,
            if report.goal_achieved { "#4CAF50" } else { "#FF9800" },
            goal_status_text,
            report.total_days,
            report.days_logged,
            (report.days_logged as f64 / report.total_days as f64 * 100.0),
            report.total_meals,
            report.streak_days,
            report.avg_calories,
            report.avg_protein_g,
            report.avg_carbs_g,
            report.avg_fat_g,
            report.calories_compliance_percent,
            report.protein_compliance_percent,
            report.carbs_compliance_percent,
            report.fat_compliance_percent,
            report.days_on_target,
            report.days_logged,
            weight_section,
            best_day_section,
            if report.goal_achieved {
                "Congratulations! You've achieved your nutrition goals for this period. Keep up the excellent work! 🎉"
            } else {
                "You're making progress! Keep tracking your meals consistently to reach your goals. 💪"
            }
        );

        let email = Message::builder()
            .from(format!("{} <{}>", self.from_name, self.from_email).parse().unwrap())
            .to(format!("{} <{}>", user.name, user.gmail).parse().unwrap())
            .subject(format!("{} {} Nutrition Report - {}", 
                goal_status_emoji, 
                report_period,
                if report.goal_achieved { "Goal Achieved!" } else { "Progress Update" }
            ))
            .header(ContentType::TEXT_HTML)
            .body(email_body)
            .map_err(|e| crate::error::AppError::InternalError(anyhow::anyhow!("Failed to build email: {}", e)))?;

        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>
            ::starttls_relay(&self.smtp_host)
            .map_err(|e| crate::error::AppError::InternalError(anyhow::anyhow!("Failed to create mailer: {}", e)))?
            .port(self.smtp_port)
            .credentials(creds)
            .build();

        mailer.send(email).await.map_err(|e| {
            tracing::error!("Failed to send report email: {}", e);
            crate::error::AppError::InternalError(anyhow::anyhow!("Failed to send email"))
        })?;

        tracing::info!("Report email sent to {}", user.gmail);

        Ok(())
    }
}
