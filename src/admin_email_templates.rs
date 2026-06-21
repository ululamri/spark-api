use chrono::{DateTime, Utc};

fn role_label(role: &str) -> &'static str {
    match role {
        "admin" | "sub_admin" => "Admin",
        "moderator" => "Moderator",
        _ => "Tim Admin",
    }
}

fn header(title: &str) -> String {
    format!(
        "Karyra Spark Admin Panel\n{title}\n{}\n",
        "-".repeat(56)
    )
}

fn footer() -> &'static str {
    "\nTerima kasih,\nTim Karyra Spark\n\n--------------------------------------------------------\nKaryra Spark\nRuang aman untuk belajar, membangun, dan berkolaborasi.\nEmail ini dikirim otomatis oleh sistem Karyra Spark. Jangan balas email ini.\nJika kamu tidak mengenali aktivitas ini, abaikan email ini atau hubungi superadmin.\n"
}

pub fn admin_invitation_email(role: &str, onboarding_url: &str, invite_code: &str, expires_at: DateTime<Utc>) -> String {
    format!(
        "{}\nHalo Sahabat Karyra,\n\nKamu menerima undangan untuk bergabung ke Karyra Spark Admin Panel sebagai {}.\n\nKode undangan kamu:\n{}\n\nUntuk menerima undangan ini, buka tautan onboarding berikut:\n{}\n\nJika halaman onboarding meminta Invite Code/Kode Undangan, salin dan masukkan kode undangan di atas.\n\nKode dan tautan ini bersifat pribadi, hanya untuk email yang diundang, dan akan kedaluwarsa pada {} UTC. Jangan teruskan kode atau tautan ini kepada orang lain.\n{}",
        header("Undangan akses admin"),
        role_label(role),
        invite_code,
        onboarding_url,
        expires_at.format("%Y-%m-%d %H:%M:%S"),
        footer()
    )
}

pub fn admin_invite_email_otp(otp: &str, expires_at: DateTime<Utc>) -> String {
    format!(
        "{}\nHalo Sahabat Karyra,\n\nGunakan kode berikut untuk melanjutkan onboarding Karyra Spark Admin Panel:\n\n{}\n\nKode ini hanya berlaku sampai {} UTC. Jika kamu tidak sedang melakukan onboarding admin, abaikan email ini dan jangan berikan kode ini kepada siapa pun.\n{}",
        header("Kode verifikasi onboarding"),
        otp,
        expires_at.format("%Y-%m-%d %H:%M:%S"),
        footer()
    )
}

pub fn password_recovery_completed_email() -> &'static str {
    "Karyra Spark Admin Panel\nPemulihan sandi selesai\n--------------------------------------------------------\n\nHalo Sahabat Karyra,\n\nSandi akun admin Karyra Spark kamu baru saja dipulihkan melalui alur pemulihan yang disetujui. Semua sesi admin lama telah dicabut agar akun tetap aman.\n\nJika aktivitas ini bukan kamu, segera hubungi superadmin.\n\nTerima kasih,\nTim Karyra Spark\n\n--------------------------------------------------------\nKaryra Spark\nRuang aman untuk belajar, membangun, dan berkolaborasi.\nEmail ini dikirim otomatis oleh sistem Karyra Spark. Jangan balas email ini.\n"
}

pub fn totp_recovery_completed_email() -> &'static str {
    "Karyra Spark Admin Panel\nPemulihan 2FA selesai\n--------------------------------------------------------\n\nHalo Sahabat Karyra,\n\nAuthenticator/2FA akun admin Karyra Spark kamu baru saja diputar ulang melalui alur pemulihan yang disetujui. Semua sesi admin lama telah dicabut agar akun tetap aman.\n\nJika aktivitas ini bukan kamu, segera hubungi superadmin.\n\nTerima kasih,\nTim Karyra Spark\n\n--------------------------------------------------------\nKaryra Spark\nRuang aman untuk belajar, membangun, dan berkolaborasi.\nEmail ini dikirim otomatis oleh sistem Karyra Spark. Jangan balas email ini.\n"
}

pub fn email_recovery_old_address_notice() -> &'static str {
    "Karyra Spark Admin Panel\nPemulihan email selesai\n--------------------------------------------------------\n\nHalo Sahabat Karyra,\n\nEmail akun admin Karyra Spark kamu baru saja diubah melalui alur pemulihan yang disetujui. Email lama ini menerima pemberitahuan sebagai catatan keamanan.\n\nJika aktivitas ini bukan kamu, segera hubungi superadmin.\n\nTerima kasih,\nTim Karyra Spark\n\n--------------------------------------------------------\nKaryra Spark\nRuang aman untuk belajar, membangun, dan berkolaborasi.\nEmail ini dikirim otomatis oleh sistem Karyra Spark. Jangan balas email ini.\n"
}

pub fn email_recovery_new_address_notice() -> &'static str {
    "Karyra Spark Admin Panel\nEmail admin aktif\n--------------------------------------------------------\n\nHalo Sahabat Karyra,\n\nEmail ini sekarang terhubung dengan akun admin Karyra Spark yang dipulihkan melalui alur keamanan. Semua sesi admin lama telah dicabut agar akun tetap aman.\n\nJika aktivitas ini bukan kamu, segera hubungi superadmin.\n\nTerima kasih,\nTim Karyra Spark\n\n--------------------------------------------------------\nKaryra Spark\nRuang aman untuk belajar, membangun, dan berkolaborasi.\nEmail ini dikirim otomatis oleh sistem Karyra Spark. Jangan balas email ini.\n"
}


pub fn admin_onboarding_completed_email(role: &str) -> String {
    format!(
        "Karyra Spark Admin Panel\nAkses admin berhasil diaktifkan\nAkun delegated admin kamu sekarang aktif.\n------------------------------------------------------------\n\nHalo Sahabat Karyra,\n\nAkun Karyra Spark Admin Panel kamu berhasil diaktifkan sebagai {}.\n\nKamu sekarang dapat masuk melalui halaman Admin Panel resmi menggunakan email, sandi, dan kode 2FA yang sudah kamu aktifkan.\n\nJika aktivasi ini bukan kamu, segera hubungi superadmin melalui kanal resmi Karyra.\n\nTerima kasih,\nTim Karyra Spark\n\n------------------------------------------------------------\nKaryra Spark\nRuang aman untuk belajar, membangun, dan berkolaborasi.\nEmail ini dikirim otomatis untuk keamanan akun dan operasional Admin Panel.\n",
        role_label(role)
    )
}
