-- Karyra Spark Directus smoke seed
-- Purpose: create one published Learn item and one published Lab item for PASS 24D bridge testing.
-- Safe to run more than once.

with course as (
  insert into karyra_courses (
    status, slug, title, summary, audience, level, sort_order
  ) values (
    'published',
    'starknet-foundation',
    'Starknet Foundation',
    'Jalur awal untuk memahami Starknet dari sudut pandang pemula lokal.',
    'pemula',
    'beginner',
    10
  )
  on conflict (slug) do update set
    status = excluded.status,
    title = excluded.title,
    summary = excluded.summary,
    audience = excluded.audience,
    level = excluded.level,
    sort_order = excluded.sort_order,
    date_updated = now()
  returning id
), lesson as (
  insert into karyra_lessons (
    course_id, status, slug, title, subtitle, summary, learning_goal,
    estimated_minutes, difficulty, sort_order, published_version, published_at
  ) values (
    (select id from course),
    'published',
    'apa-itu-starknet',
    'Apa Itu Starknet?',
    'Memahami Starknet tanpa harus langsung menjadi developer.',
    'Pelajaran ringkas yang menjelaskan Starknet dengan bahasa sederhana dan contoh dekat kehidupan sehari-hari.',
    'User memahami fungsi Starknet, kenapa scaling penting, dan apa hubungannya dengan aplikasi nyata.',
    12,
    'beginner',
    10,
    1,
    now()
  )
  on conflict (slug) do update set
    course_id = excluded.course_id,
    status = excluded.status,
    title = excluded.title,
    subtitle = excluded.subtitle,
    summary = excluded.summary,
    learning_goal = excluded.learning_goal,
    estimated_minutes = excluded.estimated_minutes,
    difficulty = excluded.difficulty,
    sort_order = excluded.sort_order,
    published_version = excluded.published_version,
    published_at = coalesce(karyra_lessons.published_at, now()),
    date_updated = now()
  returning id
)
insert into karyra_lesson_blocks (
  lesson_id, status, block_type, title, body, payload, sort_order, renderer_contract_version
) values
  (
    (select id from lesson),
    'published',
    'story',
    'Bayangkan jalan desa yang mulai ramai',
    'Saat semakin banyak orang lewat, jalan kecil mulai macet. Starknet bisa dipahami sebagai jalur tambahan yang membuat transaksi tetap lancar tanpa mengubah tujuan utama.',
    jsonb_build_object('tone', 'local_analogy'),
    10,
    1
  ),
  (
    (select id from lesson),
    'published',
    'concept',
    'Inti sederhana',
    'Starknet membantu aplikasi blockchain memproses lebih banyak aktivitas dengan biaya yang lebih masuk akal, sambil tetap terhubung dengan keamanan Ethereum.',
    jsonb_build_object('keywords', jsonb_build_array('scaling', 'validity proof', 'ethereum')),
    20,
    1
  ),
  (
    (select id from lesson),
    'published',
    'checkpoint',
    'Cek pemahaman',
    'Mengapa aplikasi membutuhkan scaling?',
    jsonb_build_object(
      'question', 'Apa alasan utama scaling dibutuhkan?',
      'choices', jsonb_build_array('Agar aplikasi bisa menangani lebih banyak aktivitas', 'Agar wallet hilang', 'Agar internet tidak dipakai'),
      'answer', 0
    ),
    30,
    1
  )
on conflict do nothing;

with runtime as (
  insert into karyra_lab_runtime_profiles (
    status, slug, title, runtime_type, sdk_profile, tool_requirements,
    allowed_commands, network_policy, filesystem_policy, command_timeout_seconds
  ) values (
    'published',
    'browser-only-foundation',
    'Browser-only Foundation',
    'browser_only',
    '',
    '[]'::jsonb,
    '[]'::jsonb,
    'disabled',
    'none',
    30
  )
  on conflict (slug) do update set
    status = excluded.status,
    title = excluded.title,
    runtime_type = excluded.runtime_type,
    sdk_profile = excluded.sdk_profile,
    tool_requirements = excluded.tool_requirements,
    allowed_commands = excluded.allowed_commands,
    network_policy = excluded.network_policy,
    filesystem_policy = excluded.filesystem_policy,
    command_timeout_seconds = excluded.command_timeout_seconds,
    date_updated = now()
  returning id
), lab as (
  insert into karyra_lab_modules (
    status, slug, title, summary, learning_goal, runtime_profile_id,
    estimated_minutes, difficulty, prerequisite_notes, published_version, published_at
  ) values (
    'published',
    'starknet-first-check',
    'Starknet First Check',
    'Lab ringan untuk memastikan user memahami konsep dasar sebelum masuk ke tool teknis.',
    'User bisa membedakan instruksi, task, hint, dan checkpoint dalam Lab Karyra.',
    (select id from runtime),
    15,
    'beginner',
    'Selesaikan pelajaran Apa Itu Starknet terlebih dahulu.',
    1,
    now()
  )
  on conflict (slug) do update set
    status = excluded.status,
    title = excluded.title,
    summary = excluded.summary,
    learning_goal = excluded.learning_goal,
    runtime_profile_id = excluded.runtime_profile_id,
    estimated_minutes = excluded.estimated_minutes,
    difficulty = excluded.difficulty,
    prerequisite_notes = excluded.prerequisite_notes,
    published_version = excluded.published_version,
    published_at = coalesce(karyra_lab_modules.published_at, now()),
    date_updated = now()
  returning id
)
insert into karyra_lab_steps (
  lab_module_id, status, step_type, title, instruction, starter_files,
  validation_mode, validation_payload, expected_output, hints, safety_notes,
  sort_order, renderer_contract_version
) values
  (
    (select id from lab),
    'published',
    'instruction',
    'Mulai dari tujuan',
    'Baca tujuan lab ini. Kamu belum perlu menjalankan terminal atau SDK.',
    '[]'::jsonb,
    'manual',
    '{}'::jsonb,
    '',
    jsonb_build_array('Fokus pada alur, bukan hafalan.'),
    '',
    10,
    1
  ),
  (
    (select id from lab),
    'published',
    'checkpoint',
    'Tandai pemahaman',
    'Tuliskan dengan bahasamu sendiri: kenapa lab berbeda dari lesson?',
    '[]'::jsonb,
    'manual',
    '{}'::jsonb,
    'Jawaban menjelaskan bahwa lab adalah tempat praktik/task, bukan sekadar membaca materi.',
    jsonb_build_array('Sebutkan kata praktik, task, atau validasi.'),
    '',
    20,
    1
  )
on conflict do nothing;
