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
        <!DOCTYPE html>
        <html>
            <head>
                <style>
                    body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; margin: 0; padding: 0; background-color: #FEF3E2; }}
                    .wrapper {{ width: 100%; table-layout: fixed; background-color: #FEF3E2; padding-bottom: 40px; }}
                    .webkit {{ max-width: 600px; margin: 0 auto; }}
                    .outer {{ margin: 0 auto; width: 100%; max-width: 600px; }}
                    .header {{ text-align: center; padding: 30px 0; }}
                    .logo-circle {{ display: inline-block; width: 50px; height: 50px; background-color: #FAB12F; border-radius: 50%; margin-bottom: 10px; }}
                    .card {{ background-color: #ffffff; border-radius: 32px; padding: 40px; box-shadow: 0 8px 32px rgba(250, 177, 47, 0.1); border: 1px solid rgba(255, 255, 255, 0.5); }}
                    h2 {{ color: #1a1a1a; margin-top: 0; font-size: 24px; font-weight: 800; letter-spacing: -0.5px; }}
                    p {{ color: #4a4a4a; font-size: 16px; line-height: 1.6; }}
                    .btn-container {{ text-align: center; margin: 35px 0; }}
                    .btn {{ background: linear-gradient(to right, #FAB12F, #FA812F); color: white !important; padding: 16px 32px; text-decoration: none; border-radius: 50px; font-weight: bold; display: inline-block; box-shadow: 0 4px 15px rgba(250, 129, 47, 0.3); }}
                    .link-text {{ color: #FA812F; word-break: break-all; font-size: 14px; }}
                    .footer {{ text-align: center; margin-top: 30px; color: #888888; font-size: 12px; }}
                </style>
            </head>
            <body>
                <div class="wrapper">
                    <div class="webkit">
                        <div class="outer">
                            <div class="header">
                                <div class="logo-circle"></div>
                                <h3 style="margin: 5px 0 0 0; color: #1a1a1a;">Alimentify</h3>
                            </div>
                            <div class="card">
                                <h2>Welcome to Alimentify! 👋</h2>
                                <p>Hello <strong>{}</strong>,</p>
                                <p>Thank you for joining us! To get started with your nutrition journey, please verify your email address.</p>
                                
                                <div class="btn-container">
                                    <a href="{}" class="btn">Verify Email Address</a>
                                </div>
                                
                                <p style="font-size: 14px; color: #666;">Or copy and paste this link into your browser:</p>
                                <p><a href="{}" class="link-text">{}</a></p>
                                
                                <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
                                
                                <p style="font-size: 13px; color: #888; margin-bottom: 0;">This link will expire in 24 hours.</p>
                                <p style="font-size: 13px; color: #888; margin-top: 5px;">If you didn't create an account, please ignore this email.</p>
                            </div>
                            <div class="footer">
                                <p>&copy; 2025 Alimentify. All rights reserved.</p>
                            </div>
                        </div>
                    </div>
                </div>
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

        let weight_section = if let (Some(start), Some(end), Some(change), Some(target)) = 
            (report.starting_weight, report.ending_weight, report.weight_change, report.target_weight) {
            format!(
                r#"
                <div style="background-color: #F8FAFC; padding: 20px; border-radius: 24px; margin: 20px 0; border: 1px solid #E2E8F0;">
                    <h3 style="color: #3B82F6; margin-top: 0; font-size: 18px;">
                        <span style="background: #EFF6FF; width: 32px; height: 32px; border-radius: 50%; display: inline-block; text-align: center; line-height: 32px; margin-right: 10px;">💪</span> 
                        Weight Progress
                    </h3>
                    <table style="width: 100%; border-collapse: collapse; margin-top: 10px;">
                        <tr>
                            <td style="padding: 8px 0; color: #64748B;">Starting Weight</td>
                            <td style="padding: 8px 0; text-align: right; font-weight: bold; color: #1E293B;">{:.1} kg</td>
                        </tr>
                        <tr>
                            <td style="padding: 8px 0; color: #64748B;">Current Weight</td>
                            <td style="padding: 8px 0; text-align: right; font-weight: bold; color: #1E293B;">{:.1} kg</td>
                        </tr>
                        <tr>
                            <td style="padding: 8px 0; color: #64748B;">Change</td>
                            <td style="padding: 8px 0; text-align: right; font-weight: bold; color: {};">{:+.1} kg</td>
                        </tr>
                        <tr>
                            <td style="padding: 8px 0; color: #64748B;">Target</td>
                            <td style="padding: 8px 0; text-align: right; font-weight: bold; color: #1E293B;">{:.1} kg</td>
                        </tr>
                    </table>
                </div>
                "#,
                start, end, 
                if change < 0.0 { "#10B981" } else { "#EF4444" },
                change, target
            )
        } else {
            String::new()
        };

        let best_day_section = if let (Some(date), Some(compliance)) = 
            (&report.best_day_date, report.best_day_compliance) {
            format!(
                r#"
                <div style="background: linear-gradient(to right, #FFF7ED, #FFFBEB); padding: 15px; border-radius: 16px; margin-top: 20px; border: 1px solid #FED7AA;">
                    <p style="margin: 0; color: #9A3412; font-size: 14px;">
                        <strong>🏆 Best Day:</strong> {} with <span style="color: #EA580C; font-weight: 800;">{:.1}%</span> compliance!
                    </p>
                </div>
                "#,
                date, compliance
            )
        } else {
            String::new()
        };

        let email_body = format!(
            r#"
            <!DOCTYPE html>
            <html>
                <head>
                    <style>
                        body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; margin: 0; padding: 0; background-color: #FEF3E2; }}
                        .wrapper {{ width: 100%; table-layout: fixed; background-color: #FEF3E2; padding-bottom: 40px; }}
                        .webkit {{ max-width: 600px; margin: 0 auto; }}
                        .outer {{ margin: 0 auto; width: 100%; max-width: 600px; }}
                        .header {{ text-align: center; padding: 30px 0; }}
                        .logo-circle {{ display: inline-block; width: 40px; height: 40px; background-color: #FAB12F; border-radius: 50%; margin-bottom: 5px; }}
                        .card {{ background-color: #ffffff; border-radius: 32px; padding: 40px; box-shadow: 0 8px 32px rgba(250, 177, 47, 0.1); border: 1px solid rgba(255, 255, 255, 0.5); }}
                        
                        h1 {{ color: #1a1a1a; font-size: 24px; font-weight: 800; margin-top: 0; letter-spacing: -0.5px; }}
                        h2 {{ color: #4a4a4a; font-size: 18px; margin-top: 30px; margin-bottom: 15px; font-weight: 700; }}
                        
                        .status-banner {{ background: linear-gradient(to right, #FAB12F, #FA812F); padding: 24px; border-radius: 24px; margin: 25px 0; text-align: center; color: white; box-shadow: 0 4px 12px rgba(250, 129, 47, 0.2); }}
                        
                        .grid-2 {{ display: table; width: 100%; border-spacing: 10px; margin: 0 -10px; }}
                        .col {{ display: table-cell; width: 50%; vertical-align: top; }}
                        
                        .metric-card {{ background-color: #F8FAFC; padding: 16px; border-radius: 20px; margin-bottom: 10px; border: 1px solid #F1F5F9; }}
                        .metric-label {{ font-size: 12px; color: #64748B; text-transform: uppercase; letter-spacing: 0.5px; font-weight: 600; display: block; margin-bottom: 4px; }}
                        .metric-value {{ font-size: 18px; font-weight: 800; color: #1E293B; }}
                        
                        .progress-container {{ margin-bottom: 15px; }}
                        .progress-bar-bg {{ background-color: #F1F5F9; height: 8px; border-radius: 4px; overflow: hidden; }}
                        .progress-bar-fill {{ height: 100%; border-radius: 4px; }}
                        
                        .footer {{ text-align: center; margin-top: 30px; color: #888888; font-size: 12px; }}
                        .btn {{ background-color: #1E293B; color: white !important; padding: 12px 24px; text-decoration: none; border-radius: 50px; font-weight: bold; display: inline-block; font-size: 14px; margin-top: 20px; }}
                    </style>
                </head>
                <body>
                    <div class="wrapper">
                        <div class="webkit">
                            <div class="outer">
                                <div class="header">
                                    <div class="logo-circle"></div>
                                    <h3 style="margin: 5px 0 0 0; color: #1a1a1a; font-family: monospace;">Alimentify</h3>
                                </div>
                                
                                <div class="card">
                                    <h1>{} {} Report</h1>
                                    <p style="color: #64748B; margin-top: 5px;">For <strong>{}</strong> • {} - {}</p>

                                    <div class="status-banner">
                                        <div style="font-size: 14px; opacity: 0.9; margin-bottom: 4px;">OVERALL STATUS</div>
                                        <div style="font-size: 24px; font-weight: 800;">{}</div>
                                    </div>

                                    <h2>📊 Summary Statistics</h2>
                                    <div class="grid-2">
                                        <div class="col">
                                            <div class="metric-card">
                                                <span class="metric-label">Logged</span>
                                                <span class="metric-value">{} <span style="font-size: 14px; color: #94A3B8; font-weight: normal;">/ {} days</span></span>
                                            </div>
                                        </div>
                                        <div class="col">
                                            <div class="metric-card">
                                                <span class="metric-label">Streak</span>
                                                <span class="metric-value">{} <span style="font-size: 14px; color: #94A3B8; font-weight: normal;">days 🔥</span></span>
                                            </div>
                                        </div>
                                    </div>

                                    <h2>🎯 Daily Averages</h2>
                                    <div class="grid-2">
                                        <div class="col">
                                            <div class="metric-card" style="background-color: #FFF7ED; border-color: #FFEDD5;">
                                                <span class="metric-label" style="color: #C2410C;">Calories</span>
                                                <span class="metric-value" style="color: #9A3412;">{:.0}</span>
                                            </div>
                                            <div class="metric-card" style="background-color: #EFF6FF; border-color: #DBEAFE;">
                                                <span class="metric-label" style="color: #1D4ED8;">Protein</span>
                                                <span class="metric-value" style="color: #1E40AF;">{:.1}g</span>
                                            </div>
                                        </div>
                                        <div class="col">
                                            <div class="metric-card" style="background-color: #F0FDF4; border-color: #DCFCE7;">
                                                <span class="metric-label" style="color: #15803D;">Carbs</span>
                                                <span class="metric-value" style="color: #166534;">{:.1}g</span>
                                            </div>
                                            <div class="metric-card" style="background-color: #FAF5FF; border-color: #F3E8FF;">
                                                <span class="metric-label" style="color: #7E22CE;">Fat</span>
                                                <span class="metric-value" style="color: #6B21A8;">{:.1}g</span>
                                            </div>
                                        </div>
                                    </div>

                                    <h2>✅ Goal Compliance</h2>
                                    
                                    <div class="progress-container">
                                        <div style="display: flex; justify-content: space-between; margin-bottom: 5px; font-size: 14px; color: #475569;">
                                            <span>Calories</span>
                                            <span style="font-weight: bold;">{:.1}%</span>
                                        </div>
                                        <div class="progress-bar-bg">
                                            <div class="progress-bar-fill" style="width: {:.1}%; background-color: #F97316;"></div>
                                        </div>
                                    </div>

                                    <div class="progress-container">
                                        <div style="display: flex; justify-content: space-between; margin-bottom: 5px; font-size: 14px; color: #475569;">
                                            <span>Protein</span>
                                            <span style="font-weight: bold;">{:.1}%</span>
                                        </div>
                                        <div class="progress-bar-bg">
                                            <div class="progress-bar-fill" style="width: {:.1}%; background-color: #3B82F6;"></div>
                                        </div>
                                    </div>

                                    <div class="progress-container">
                                        <div style="display: flex; justify-content: space-between; margin-bottom: 5px; font-size: 14px; color: #475569;">
                                            <span>Carbs</span>
                                            <span style="font-weight: bold;">{:.1}%</span>
                                        </div>
                                        <div class="progress-bar-bg">
                                            <div class="progress-bar-fill" style="width: {:.1}%; background-color: #22C55E;"></div>
                                        </div>
                                    </div>

                                    {}

                                    {}

                                    <div style="text-align: center; margin-top: 40px;">
                                        <p style="color: #475569; font-style: italic;">"{}"</p>
                                        <a href="https://alimentify.app/my/reports" class="btn">View Full Report</a>
                                    </div>
                                </div>

                                <div class="footer">
                                    <p>You received this email because you enabled nutrition reports in your settings.</p>
                                    <p>&copy; 2025 Alimentify. All rights reserved.</p>
                                </div>
                            </div>
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
            if report.goal_achieved { "GOAL ACHIEVED" } else { "IN PROGRESS" },
            report.days_logged,
            report.total_days,
            report.streak_days,
            report.avg_calories,
            report.avg_protein_g,
            report.avg_carbs_g,
            report.avg_fat_g,
            report.calories_compliance_percent,
            if report.calories_compliance_percent > 100.0 { 100.0 } else { report.calories_compliance_percent },
            report.protein_compliance_percent,
            if report.protein_compliance_percent > 100.0 { 100.0 } else { report.protein_compliance_percent },
            report.carbs_compliance_percent,
            if report.carbs_compliance_percent > 100.0 { 100.0 } else { report.carbs_compliance_percent },
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
