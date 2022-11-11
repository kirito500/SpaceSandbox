![alt text](https://github.com/rewin123/SpaceSandbox/blob/main/image.png?raw=true)

Полезные ссылки:
1. https://github.com/bombomby/optick - профайлер для игр
2. https://crates.io/crates/profiling - обертка профайлеров для rust
3. https://crates.io/crates/texture-synthesis - интересный генератор текстур
4. https://github.com/tree-sitter/tree-sitter - инкрементальный парсер кода для подсветки

Полезные ссылки про визуализацию:

1. Voxel based near-field global illumination (2011) - https://dl.acm.org/doi/pdf/10.1145/1944745.1944763

Разработка ведется в стиле функционал-рефакторинг

"Функционал" - увеличиваем функционал кода
"Рефакторинг" - чистка кода и определяем как можно уменьшить объем кода, выделив:
1. Общие куски кода, которые можно представить в виде функции
2. Уменьшение объема и модульности кода за счет абстракций на основе trait

Оба этапа должны идти один за другим. После добавления, или набора добавлений, обязательно должен идти рефакторинг. 
Новый код не должен появляться, хотя бы без одного рефакторинга.

Ограничение на стиль кода:

1. Каждый файл по объему не должен превышать 400 строчек кода.
2. Каждая функция кода не должна превышать 100 строчек кода (желательно не превышать 40 строчек кода)
3. В каждой папке должно быть не более 10 файлов и не более 10 папок
4. Нужно следовать общепринятому code style принятому в Rust

Все коммиты, увеличиваюшие функционал должны идти с аттрибутом "add:".
Все коммиты, производящие рефактор, должны идти с аттрибутом "clean:".

Оценивать объем кода следует с помощью команды: loc
Эта команда подсчитывает количество строк кода в проекта.
Устанавливается с помощью комманды "cargo install loc".
В линуксе бинарник команды лежит по пути "/home/user/.cargo/bin".
