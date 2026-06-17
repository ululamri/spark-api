-- PASS 17D: seed Admin CMS from the current source-controlled Spark content.
-- This is additive and non-destructive. Existing CMS items are not overwritten.

with source(kind, slug, title, payload) as (
  values
    ('core_lesson', 'why-blockchain', 'Kenapa Kita Butuh Blockchain?', $json$
      {
        "source": "spark-content.ts",
        "source_id": "why-blockchain",
        "module_id": "blockchain-foundation",
        "module_level": 1,
        "module_title": "Fondasi Blockchain",
        "summary": "Mulai dari masalah kepercayaan digital dengan contoh sehari-hari.",
        "estimated_minutes": 8,
        "mode_hint": ["beginner", "guided"],
        "checkpoint": "Pengguna bisa menjelaskan masalah trust dengan bahasa sederhana.",
        "body": [
          "Bayangkan ada catatan iuran komunitas yang hanya disimpan oleh satu orang. Jika catatan itu hilang, rusak, atau diubah sepihak, anggota lain sulit membuktikan versi mana yang benar.",
          "Blockchain lahir dari masalah kepercayaan digital seperti ini. Ia membuat catatan bersama yang dapat diperiksa banyak pihak, sehingga perubahan tidak hanya bergantung pada satu penjaga data.",
          "Untuk pemula, bagian terpenting bukan harga token atau janji cepat kaya. Bagian terpenting adalah memahami bahwa blockchain membantu orang mengecek asal-usul, urutan, dan keutuhan sebuah catatan.",
          "Di Spark, blockchain menjadi fondasi sebelum membahas cryptocurrency, wallet, Web3, dan Starknet. Urutannya sengaja dibuat pelan agar pengguna tidak langsung masuk ke risiko teknis tanpa memahami alasan dasarnya."
        ],
        "checkpoint_question": "Apa masalah utama yang ingin dijawab oleh blockchain?",
        "checkpoint_options": [
          {"id":"trust","label":"Membantu banyak pihak memverifikasi catatan bersama","correct":true,"feedback":"Benar. Fokus awal blockchain adalah trust dan verifikasi."},
          {"id":"profit","label":"Membuat semua orang cepat kaya","feedback":"Belum tepat. Spark tidak memulai dari spekulasi, tetapi dari pemahaman trust."},
          {"id":"password","label":"Mengganti semua password internet","feedback":"Belum tepat. Blockchain bisa terkait identitas, tetapi bukan sekadar pengganti password."}
        ],
        "glossary_terms": [
          {"term":"Blockchain","simple":"Catatan digital bersama yang sulit diubah sepihak.","technical":"Distributed ledger dengan mekanisme konsensus."},
          {"term":"Trust","simple":"Rasa percaya bahwa catatan atau aksi bisa diperiksa kebenarannya."}
        ]
      }
    $json$::jsonb),
    ('core_lesson', 'shared-ledger', 'Catatan Bersama yang Sulit Diubah', $json$
      {
        "source": "spark-content.ts",
        "source_id": "shared-ledger",
        "module_id": "blockchain-foundation",
        "module_level": 1,
        "module_title": "Fondasi Blockchain",
        "summary": "Memahami ledger tanpa masuk ke istilah teknis berlebihan.",
        "estimated_minutes": 10,
        "mode_hint": ["beginner", "guided"],
        "checkpoint": "Pengguna memahami kenapa data bersama perlu disepakati.",
        "body": [
          "Ledger bisa dibayangkan sebagai buku catatan. Bedanya, dalam blockchain catatan itu tidak hanya disimpan di satu tempat, melainkan direplikasi dan diperiksa oleh banyak peserta jaringan.",
          "Ketika banyak pihak memegang salinan catatan yang sama, perubahan sepihak menjadi lebih mudah terdeteksi. Jaringan perlu menyepakati urutan data agar semua pihak punya gambaran yang konsisten.",
          "Transparan bukan berarti semua identitas pribadi terbuka. Yang penting bagi pemula adalah memahami bahwa aktivitas di jaringan biasanya meninggalkan jejak yang dapat diperiksa, sehingga kehati-hatian sangat penting.",
          "Sebelum masuk ke wallet atau aplikasi, pengguna perlu tahu bahwa setiap aksi onchain bukan sekadar klik biasa. Aksi itu masuk ke catatan bersama dan dapat berdampak pada akun, aset, atau reputasi digital."
        ],
        "checkpoint_question": "Kenapa catatan bersama lebih kuat daripada catatan satu pihak?",
        "checkpoint_options": [
          {"id":"shared","label":"Karena banyak pihak bisa ikut memeriksa dan menyepakati catatan","correct":true,"feedback":"Benar. Verifikasi bersama membuat manipulasi lebih sulit."},
          {"id":"secret","label":"Karena catatannya harus selalu rahasia","feedback":"Tidak selalu. Banyak blockchain justru transparan, meskipun identitas bisa tetap terlindungi."}
        ],
        "glossary_terms": [
          {"term":"Ledger","simple":"Buku catatan transaksi atau perubahan data."},
          {"term":"Verifikasi","simple":"Proses memeriksa apakah sesuatu benar atau valid."}
        ]
      }
    $json$::jsonb),
    ('core_lesson', 'what-is-token', 'Apa Itu Token?', $json$
      {
        "source": "spark-content.ts",
        "source_id": "what-is-token",
        "module_id": "cryptocurrency-basics",
        "module_level": 2,
        "module_title": "Cryptocurrency",
        "summary": "Token sebagai representasi nilai/akses dalam jaringan blockchain.",
        "estimated_minutes": 9,
        "mode_hint": ["beginner", "guided"],
        "checkpoint": "Pengguna bisa membedakan blockchain dan token.",
        "body": [
          "Token adalah unit digital yang hidup di atas jaringan blockchain. Token bisa mewakili nilai, akses, hak, poin, biaya jaringan, atau fungsi tertentu di sebuah aplikasi.",
          "Spark tidak memperkenalkan token sebagai ajakan trading. Untuk pemula, token lebih aman dipahami sebagai bagian dari cara aplikasi blockchain mencatat kepemilikan, izin, atau aktivitas.",
          "Ada token yang dipakai untuk membayar biaya jaringan, ada token yang mewakili aset, dan ada token yang dipakai sebagai tanda partisipasi. Fungsi token selalu perlu dibaca dari konteksnya, bukan dari nama atau hype-nya saja.",
          "Sebelum menyentuh token sungguhan, pengguna perlu belajar membaca risiko: siapa penerbitnya, untuk apa token dipakai, apakah perlu signature, dan apakah aksi itu memakai testnet atau aset utama."
        ],
        "checkpoint_question": "Dalam Spark, token sebaiknya dipahami sebagai apa terlebih dahulu?",
        "checkpoint_options": [
          {"id":"representation","label":"Representasi nilai, akses, atau utilitas di jaringan","correct":true,"feedback":"Benar. Kita pahami token secara fungsional dulu."},
          {"id":"gambling","label":"Alat untuk spekulasi cepat","feedback":"Tidak tepat. Spark menghindari framing spekulatif untuk pemula."}
        ],
        "glossary_terms": [
          {"term":"Token","simple":"Unit digital yang bisa mewakili nilai, akses, hak, atau utilitas."},
          {"term":"Transaksi","simple":"Aksi yang dicatat di jaringan blockchain."}
        ]
      }
    $json$::jsonb),
    ('core_lesson', 'wallet-is-not-bank', 'Wallet Bukan Rekening Bank', $json$
      {
        "source": "spark-content.ts",
        "source_id": "wallet-is-not-bank",
        "module_id": "wallet-security",
        "module_level": 3,
        "module_title": "Wallet & Keamanan",
        "summary": "Mengenal perbedaan wallet, akun, address, dan tanggung jawab pengguna.",
        "estimated_minutes": 12,
        "mode_hint": ["beginner", "guided"],
        "checkpoint": "Pengguna tahu kenapa seed phrase tidak boleh dibagikan.",
        "body": [
          "Wallet bukan rekening bank biasa. Wallet adalah alat untuk mengelola akses ke akun blockchain, sedangkan kendali utamanya bergantung pada kunci rahasia seperti seed phrase atau private key.",
          "Jika seseorang mendapatkan seed phrase, ia dapat mengambil alih akses. Karena itu seed phrase tidak boleh dikirim lewat chat, disimpan sebagai screenshot, dimasukkan ke form sembarang, atau dibagikan kepada orang yang mengaku sebagai admin.",
          "Connect wallet, approve, dan sign adalah tiga pengalaman yang sering terlihat mirip oleh pemula, padahal risikonya berbeda. Connect biasanya mengenalkan alamat, sedangkan approve atau sign bisa memberi izin atau menyetujui aksi tertentu.",
          "Di Spark, pengguna dilatih mengenali bahasa risiko sebelum memakai wallet sungguhan. Tujuannya bukan menakut-nakuti, tetapi membangun kebiasaan aman: baca konteks, cek tujuan aksi, dan jangan tanda tangan jika belum paham."
        ],
        "checkpoint_question": "Apa prinsip keamanan paling penting saat memakai wallet?",
        "checkpoint_options": [
          {"id":"seed","label":"Jangan pernah membagikan seed phrase/private key","correct":true,"feedback":"Benar. Ini prinsip utama sebelum praktik apa pun."},
          {"id":"screenshot","label":"Simpan seed phrase di screenshot agar mudah dicari","feedback":"Berbahaya. Screenshot bisa tersinkron atau bocor."}
        ],
        "glossary_terms": [
          {"term":"Wallet","simple":"Aplikasi untuk mengelola akses ke akun/aset blockchain."},
          {"term":"Seed phrase","simple":"Kumpulan kata rahasia untuk memulihkan wallet. Jangan dibagikan."},
          {"term":"Signature","simple":"Tanda persetujuan digital terhadap sebuah aksi."}
        ]
      }
    $json$::jsonb),
    ('core_lesson', 'web3-interactions', 'Cara Berinteraksi dengan Aplikasi Web3', $json$
      {
        "source": "spark-content.ts",
        "source_id": "web3-interactions",
        "module_id": "web3-apps",
        "module_level": 4,
        "module_title": "Web3 & Aplikasi",
        "summary": "Menghubungkan konsep wallet dengan pengalaman menggunakan aplikasi.",
        "estimated_minutes": 10,
        "mode_hint": ["guided", "explorer"],
        "checkpoint": "Pengguna memahami connect wallet tidak sama dengan mengirim aset.",
        "body": [
          "Aplikasi Web3 berbeda dari aplikasi biasa karena sebagian interaksinya dapat terhubung dengan wallet dan jaringan blockchain. Pengguna tidak hanya membuat akun, tetapi membawa alamat wallet sebagai identitas atau akses.",
          "Connect wallet bukan otomatis mengirim aset. Namun connect tetap perlu dipahami karena aplikasi bisa membaca alamat publik dan menampilkan fitur berdasarkan alamat itu.",
          "Interaksi yang lebih serius biasanya meminta signature atau transaksi. Di tahap ini pengguna harus membaca pesan, memahami tujuan aksi, dan memastikan berada di situs yang benar sebelum menyetujui apa pun.",
          "Spark menempatkan Web3 setelah wallet safety agar pengguna tidak hanya tahu tombol mana yang diklik, tetapi juga tahu kapan harus berhenti, bertanya, atau membatalkan aksi yang terasa tidak jelas."
        ],
        "checkpoint_question": "Apa arti connect wallet secara sederhana?",
        "checkpoint_options": [
          {"id":"identity","label":"Memberi aplikasi cara mengenali alamat wallet kita","correct":true,"feedback":"Benar. Connect wallet bukan otomatis mengirim aset."},
          {"id":"transfer","label":"Otomatis mengirim semua aset ke aplikasi","feedback":"Tidak tepat. Transfer butuh aksi/signature tambahan."}
        ],
        "glossary_terms": [
          {"term":"Web3","simple":"Cara memakai aplikasi yang terhubung dengan wallet dan jaringan blockchain."},
          {"term":"Connect wallet","simple":"Menghubungkan alamat wallet ke aplikasi."}
        ]
      }
    $json$::jsonb),
    ('core_lesson', 'starknet-first-look', 'Pandangan Pertama ke Starknet', $json$
      {
        "source": "spark-content.ts",
        "source_id": "starknet-first-look",
        "module_id": "starknet-entry",
        "module_level": 5,
        "module_title": "Starknet",
        "summary": "Mengenal Starknet sebagai bagian dari perjalanan blockchain, bukan loncatan teknis mendadak.",
        "estimated_minutes": 12,
        "mode_hint": ["guided", "explorer"],
        "checkpoint": "Pengguna memahami kenapa Spark membawa mereka ke Starknet secara bertahap.",
        "body": [
          "Starknet adalah jaringan Layer 2 yang membantu aplikasi blockchain berjalan lebih scalable dengan tetap terhubung pada keamanan Ethereum melalui bukti kriptografi. Untuk pemula, cukup pahami dulu bahwa Starknet adalah jalur ekosistem yang membutuhkan kesiapan sebelum praktik.",
          "Di Starknet, akun pengguna berbentuk smart contract account. Ini membuka ruang untuk account abstraction, yaitu akun yang dapat punya logika lebih fleksibel dibanding model wallet sederhana.",
          "Fleksibilitas ini menarik, tetapi pemula tetap perlu disiplin membaca risiko. Wallet, signature, transaksi, biaya jaringan, testnet, dan explorer tetap harus dipahami secara bertahap.",
          "Spark mengenalkan Starknet setelah fondasi blockchain, token, wallet, dan Web3 karena tujuan utamanya adalah readiness. Pengguna diarahkan untuk memahami dulu, berlatih aman, lalu mengeksplorasi Hub dan ekosistem Starknet dengan lebih percaya diri."
        ],
        "checkpoint_question": "Kenapa Spark mengenalkan Starknet secara bertahap?",
        "checkpoint_options": [
          {"id":"safe","label":"Agar pengguna punya fondasi sebelum praktik teknis","correct":true,"feedback":"Benar. Spark mengutamakan readiness sebelum eksplorasi teknis."},
          {"id":"random","label":"Karena Starknet tidak berhubungan dengan blockchain","feedback":"Tidak tepat. Starknet adalah bagian dari ekosistem blockchain."}
        ],
        "glossary_terms": [
          {"term":"Starknet","simple":"Ekosistem blockchain yang menjadi fokus eksplorasi Spark."},
          {"term":"Testnet","simple":"Jaringan latihan untuk belajar tanpa memakai aset utama."},
          {"term":"Cairo","simple":"Bahasa/teknologi yang digunakan dalam pengembangan Starknet."}
        ]
      }
    $json$::jsonb),
    ('core_lesson', 'cairo-gentle-intro', 'Cairo: Kenalan, Bukan Langsung Coding', $json$
      {
        "source": "spark-content.ts",
        "source_id": "cairo-gentle-intro",
        "module_id": "starknet-entry",
        "module_level": 5,
        "module_title": "Starknet",
        "summary": "Jembatan sebelum pengguna melihat kode atau metrik jaringan.",
        "estimated_minutes": 11,
        "mode_hint": ["explorer"],
        "checkpoint": "Pengguna tahu bahwa Cairo adalah bagian teknis yang bisa dipelajari setelah fondasi siap.",
        "body": [
          "Cairo adalah bahasa dan ekosistem pengembangan yang digunakan dalam dunia Starknet. Pemula tidak harus langsung menulis kode, tetapi penting mengenal bahwa aplikasi Starknet dibangun dengan alat teknis yang bisa dipelajari bertahap.",
          "Cara paling aman untuk mengenalkan Cairo kepada non-teknikal learner adalah sebagai peta, bukan ujian. Mereka cukup tahu bahwa smart contract berisi aturan, fungsi, dan data yang menentukan bagaimana sebuah aplikasi berjalan.",
          "Setelah pengguna lebih siap, Cairo dapat menjadi pintu menuju jalur builder: membaca contoh kode, memahami contract, mencoba tool seperti Scarb atau Starknet Foundry, lalu masuk ke latihan teknis yang lebih serius.",
          "Di Spark, lesson Cairo ini berfungsi sebagai jembatan. Ia memberi konteks bahwa Starknet bukan hanya tempat memakai aplikasi, tetapi juga ruang untuk membangun, berkontribusi, dan memahami teknologi secara lebih dalam."
        ],
        "checkpoint_question": "Bagaimana pemula sebaiknya melihat Cairo?",
        "checkpoint_options": [
          {"id":"bridge","label":"Sebagai tahap teknis lanjutan setelah fondasi siap","correct":true,"feedback":"Benar. Cairo tidak harus muncul sebelum pengguna siap."},
          {"id":"first","label":"Sebagai hal pertama yang wajib dipelajari semua pemula","feedback":"Belum tepat. Pemula perlu memahami fondasi dulu."}
        ],
        "glossary_terms": [
          {"term":"Cairo","simple":"Bagian teknis pengembangan Starknet yang bisa dipelajari bertahap."},
          {"term":"Explorer mode","simple":"Mode untuk pengguna yang siap melihat detail teknis."}
        ]
      }
    $json$::jsonb),
    ('lab', 'safe-wallet-check', 'Simulasi Cek Wallet Aman', $json$
      {
        "source": "spark-content.ts",
        "source_id": "safe-wallet-check",
        "summary": "Latihan mengenali aksi aman tanpa memakai aset sungguhan.",
        "difficulty": "safe",
        "action": "Mulai Simulasi",
        "estimated_minutes": 8,
        "readiness_hint": "Cocok untuk pemula sebelum connect wallet.",
        "steps": ["Baca skenario", "Kenali permintaan berbahaya", "Pilih tindakan aman", "Simpan hasil ke Passport"]
      }
    $json$::jsonb),
    ('lab', 'testnet-readiness', 'Testnet Readiness', $json$
      {
        "source": "spark-content.ts",
        "source_id": "testnet-readiness",
        "summary": "Checklist sebelum mencoba aksi testnet di ekosistem Starknet.",
        "difficulty": "guided",
        "action": "Cek Readiness",
        "estimated_minutes": 12,
        "readiness_hint": "Direkomendasikan setelah Wallet & Keamanan.",
        "steps": ["Cek pemahaman wallet", "Cek risiko signature", "Simulasi biaya jaringan", "Tandai siap testnet"]
      }
    $json$::jsonb),
    ('lab', 'cairo-preview', 'Cairo Preview', $json$
      {
        "source": "spark-content.ts",
        "source_id": "cairo-preview",
        "summary": "Melihat contoh kode dengan jembatan penjelasan untuk mode penjelajah.",
        "difficulty": "technical",
        "action": "Buka Preview",
        "estimated_minutes": 15,
        "readiness_hint": "Untuk mode penjelajah. Pemula akan melihat bridge warning dulu.",
        "requires_bridge": true,
        "steps": ["Baca peringatan teknis", "Lihat contoh kode sederhana", "Pahami istilah Cairo", "Simpan sebagai eksplorasi"]
      }
    $json$::jsonb)
), normalized as (
  select kind::text,
         slug::text,
         title::text,
         payload::jsonb,
         md5('admin-cms-item:' || kind || ':' || slug)::uuid as item_id,
         md5('admin-cms-revision:' || kind || ':' || slug || ':1')::uuid as revision_id,
         md5('admin-cms-event:' || kind || ':' || slug || ':seed-v1')::uuid as event_id
  from source
), inserted_items as (
  insert into admin_cms_items (
    id, kind, slug, title, status, current_revision_id,
    created_by_kind, updated_by_kind, published_at, metadata
  )
  select item_id,
         kind,
         slug,
         title,
         'published',
         revision_id,
         'system_seed',
         'system_seed',
         now(),
         jsonb_build_object('source', 'spark-content.ts', 'seed_version', 1)
  from normalized
  on conflict (kind, slug) do nothing
  returning id
), target_items as (
  select item.id,
         item.kind,
         item.slug,
         normalized.title,
         normalized.payload,
         normalized.revision_id,
         normalized.event_id
  from normalized
  join admin_cms_items item on item.kind = normalized.kind and item.slug = normalized.slug
), inserted_revisions as (
  insert into admin_cms_revisions (
    id, item_id, version, payload, summary, created_by_kind
  )
  select revision_id,
         id,
         1,
         payload,
         'Initial import from source-controlled spark-content.ts.',
         'system_seed'
  from target_items
  on conflict (item_id, version) do nothing
  returning id, item_id
), synced_items as (
  update admin_cms_items item
  set current_revision_id = target.revision_id,
      status = case when item.status = 'draft' and item.current_revision_id is null then 'published' else item.status end,
      published_at = case when item.published_at is null and item.current_revision_id is null then now() else item.published_at end,
      updated_at = now(),
      metadata = coalesce(item.metadata, '{}'::jsonb) || jsonb_build_object('source_seed_available', true)
  from target_items target
  where item.id = target.id
    and item.current_revision_id is null
  returning item.id, target.revision_id
)
insert into admin_cms_publish_events (
  id, item_id, revision_id, action, actor_kind, reason, metadata
)
select target.event_id,
       target.id,
       target.revision_id,
       'publish',
       'system_seed',
       'Initial published import from source-controlled spark-content.ts.',
       jsonb_build_object('source', 'spark-content.ts', 'seed_version', 1)
from target_items target
where exists (select 1 from inserted_revisions rev where rev.item_id = target.id)
on conflict (id) do nothing;
