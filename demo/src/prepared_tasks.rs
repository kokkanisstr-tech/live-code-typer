// The code files were transcribed from the 25 task blocks in the supplied DOCX.
// Keep each task in its own file so its exact text remains reviewable.
pub const PREPARED_TASKS: [(&str, &str); DEMO_TASK_COUNT] = [
    ("Колледждегі сабақ кестесі", include_str!("../tasks/01.cs")),
    ("Кітапхана жүйесі", include_str!("../tasks/02.cs")),
    ("Компьютерлік ойын кейіпкерлері", include_str!("../tasks/03.cs")),
    ("Құжаттарды өңдеу жүйесі", include_str!("../tasks/04.cs")),
    ("Медициналық қызметтер жүйесі", include_str!("../tasks/05.cs")),
    ("Электрондық құрылғылар", include_str!("../tasks/06.cs")),
    ("Колледж қызметкерлері", include_str!("../tasks/07.cs")),
    ("Файлдарды сақтау жүйесі", include_str!("../tasks/08.cs")),
    ("Қоғамдық көлік жүйесі", include_str!("../tasks/09.cs")),
    ("Онлайн курстар жүйесі", include_str!("../tasks/10.cs")),
    ("Роботтар жүйесі", include_str!("../tasks/11.cs")),
    ("Әуежайдағы рейстер", include_str!("../tasks/12.cs")),
    ("Спорттық жарыстар", include_str!("../tasks/13.cs")),
    ("Музыкалық аспаптар", include_str!("../tasks/14.cs")),
    ("Қонақүй бөлмелерін басқару", include_str!("../tasks/15.cs")),
    ("Электрондық төлем жүйесі", include_str!("../tasks/16.cs")),
    ("Желілік құрылғылар", include_str!("../tasks/17.cs")),
    ("Өндірістік жабдықтар", include_str!("../tasks/18.cs")),
    ("Туристік саяхаттар", include_str!("../tasks/19.cs")),
    ("Хабарламаларды жіберу жүйесі", include_str!("../tasks/20.cs")),
    ("Қоршаған ортаны бақылау датчиктері", include_str!("../tasks/21.cs")),
    ("Қоймадағы жабдықтарды есепке алу", include_str!("../tasks/22.cs")),
    ("Банк хабарламалары", include_str!("../tasks/23.cs")),
    ("Автоматты тестілеу жүйесі", include_str!("../tasks/24.cs")),
    ("Колледждің ақпараттық жүйесі", include_str!("../tasks/25.cs")),
];

fn prepared_task_snippets() -> Vec<Snippet> {
    PREPARED_TASKS
        .iter()
        .enumerate()
        .map(|(index, (title, code))| {
            let mut snippet = Snippet::new(format!("Задание {} · {title}", index + 1), "C#", *code);
            snippet.pinned = true;
            snippet
        })
        .collect()
}
