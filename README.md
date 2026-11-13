# LiveCode Typer

Two versions are included:

- **Original** — portable desktop app and room server in original/.
- **Demo** — separate copy with 25 prepared C# tasks and Alt+1 through Alt+7 shortcuts for the first seven in demo/.

Download ready-to-run packages from [GitHub Releases](https://github.com/kokkanisstr-tech/live-code-typer/releases). Each package contains the app, room server, and a short start guide.

## Quick start

1. Choose Original or Demo and the package for your computer in Releases.
2. Unpack it. On Windows, start live-code-typer.exe or live-code-typer-demo.exe. On macOS, open the matching .app.
3. Paste or choose text in the app, turn typing on, and click the editor field where the code should appear.
4. Press ordinary keys to advance through the prepared text. Spaces and line breaks are entered automatically.
5. Default controls are Ctrl+Alt+Space to toggle typing and Ctrl+Alt+R to restart; change them in Settings. Windows reserves Alt+Space for its window menu.
6. In the Demo, Alt+1–Alt+7 select tasks. Mouse button 4 moves forward and button 5 moves back; switching tasks keeps typing active.

The macOS app needs Accessibility and Input Monitoring permissions. Builds are ad-hoc signed, not notarized by Apple, so macOS may ask you to approve the first launch.

## Install from a terminal

The installer downloads the latest matching release, places it in your user folder, and starts the app. It does not need administrator access.

Windows PowerShell, Original:

    irm https://raw.githubusercontent.com/kokkanisstr-tech/live-code-typer/main/install.ps1 | iex

Windows PowerShell, Demo:

    & ([scriptblock]::Create((irm https://raw.githubusercontent.com/kokkanisstr-tech/live-code-typer/main/install.ps1))) -Variant demo

macOS, Original:

    curl -fsSL https://raw.githubusercontent.com/kokkanisstr-tech/live-code-typer/main/install.sh | sh

macOS, Demo:

    curl -fsSL https://raw.githubusercontent.com/kokkanisstr-tech/live-code-typer/main/install.sh | sh -s -- demo

On Windows, the installer adds its folder to your user PATH. Open a new terminal to run the app by its command name. On macOS, it installs under ~/Applications/LiveCodeTyper/. Running the command again updates the app and keeps its saved data.

## Online room

Run the included room-server launcher on one computer. Participants on the same trusted Wi-Fi or local network open the invitation link and connect their desktop apps. The room can assign tasks or send custom text to a participant.

The room server uses plain HTTP and is intended for a trusted local network. Do not expose it directly to the public internet.

## Build from source

Install Rust 1.95 or later.

    cd original
    cargo build --release --locked --bins

For the demo:

    cd demo
    cargo build --release --locked --bins --example demo_controls_e2e

See each folder's README for platform setup and end-to-end verification commands.

## Releases

Version tags build Windows x64, macOS Intel, and Apple Silicon packages, run supported headless E2E checks, and publish downloadable files:

    git tag original-v0.3.1
    git push origin original-v0.3.1

    git tag demo-v0.4.0
    git push origin demo-v0.4.0

## Қазақша

Негізгі бағдарлама original/ ішінде, жеті дайын C# тапсырмасы бар демо demo/ ішінде. Дайын файлдарды Releases бөлімінен жүктеп алыңыз. Windows-та архивті ашып .exe файлын, Mac-та құрылғыңызға сәйкес .app файлын іске қосыңыз.

Терминалдан орнату үшін жоғарыдағы Windows немесе Mac командасын орындаңыз. Негізгі нұсқа әдепкіде орнатылады; демо командасында demo параметрін қолданыңыз. Орнатқыш соңғы нұсқаны жүктеп, қолданушы папкасына орнатады.

Дайын кодты таңдаңыз, теруді қосыңыз да, код шығатын редакторды белсенді етіңіз. Кездейсоқ пернелер мәтінді бір таңбадан жазады; пробел мен жаңа жол автоматты түрде енгізіледі. Әдепкі басқару пернелері: Ctrl+Alt+Space — теруді қосу/тоқтату, Ctrl+Alt+R — басына қайтару; оларды баптаулардан өзгертуге болады. Windows-та Alt+Space жүйелік мәзірді ашады. Демо нұсқада Alt+1–Alt+7 тапсырманы таңдайды; тінтуірдің 4-бүйір батырмасы алға, 5-батырмасы артқа ауыстырады. Тапсырманы ауыстырғанда теру тоқтамайды.

Онлайн бөлме сенімді Wi-Fi немесе жергілікті желіге арналған. Серверді ашық интернетке шығармаңыз. Mac жүйесінде Accessibility және Input Monitoring рұқсаттары қажет.

## Русский

Основная программа находится в original/, демо с 25 заданиями C# — в demo/. Готовые файлы скачиваются в Releases. В Windows распакуйте архив и запустите .exe, на Mac выберите .app для своего процессора.

Для установки из терминала скопируйте команду Windows или Mac выше. По умолчанию устанавливается основная версия; для демо укажите demo. Установщик скачивает последнюю версию, ставит её в пользовательскую папку и запускает.

Выберите код, включите набор и перейдите в нужное поле редактора. Нажатия обычных клавиш последовательно вводят подготовленный текст; пробелы и переносы строк программа вводит сама. По умолчанию Ctrl+Alt+Space включает или останавливает набор, а Ctrl+Alt+R сбрасывает текст; сочетания можно изменить в настройках. В Windows Alt+Space открывает системное меню. В демо Alt+1–Alt+7 выбирают задание, боковая кнопка мыши 4 переключает вперёд, кнопка 5 — назад. Переключение не останавливает набор.

Онлайн-комната рассчитана на доверенную локальную сеть. Не открывайте сервер в публичный интернет. На Mac приложению понадобятся разрешения Accessibility и Input Monitoring.
