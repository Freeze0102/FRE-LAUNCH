"""Complete missing Ukrainian translations in the pinned Modrinth desktop fork.

Existing Ukrainian strings are never overwritten. English is used only as
a fallback for keys not supplied here, preserving ICU variable placeholders.
"""
import json
from pathlib import Path

APP = {
"app.file-download.canceling":"Скасування завантаження…",
"app.file-download.preparing":"Підготовка завантаження…",
"app.file-download.waiting":"Очікування відповіді сервера…",
"app.instance.confirm-delete.bulk-admonition-body":"Усі дані цих {count} збірок буде назавжди видалено, включно зі світами, налаштуваннями та встановленим вмістом.",
"app.instance.confirm-delete.bulk-delete-button":"Видалити {count} збірок",
"app.instance.confirm-delete.bulk-header":"Видалити {count} збірок",
"app.modal.install-to-play.unavailable-description":"Версія збірки, яку використовує сервер, зараз недоступна для завантаження. Можливо, її приховано або вона проходить перевірку. Спробуйте пізніше або зверніться до власника сервера.",
"app.modal.install-to-play.unavailable-header":"Збірка цього сервера недоступна",
"app.settings.sidebar.label.instance-defaults":"Типові налаштування збірок",
"app.settings.sidebar.label.system":"Система",
"app.settings.tabs.java-and-memory":"Java та пам’ять",
"app.settings.tabs.launch-hooks":"Команди запуску",
"app.settings.tabs.resources":"Ресурси",
"app.settings.tabs.syncing":"Синхронізація",
"app.settings.tabs.troubleshooting":"Усунення проблем",
"app.settings.tabs.window":"Вікно",
"app.settings.troubleshooting.clear-cache":"Очистити кеш",
"app.settings.troubleshooting.clear-cache.confirm.title":"Очистити кеш застосунку?",
"app.settings.troubleshooting.report-ad.copied":"Скопійовано",
"app.settings.troubleshooting.report-ad.copy":"Скопіювати дані реклами",
"app.settings.troubleshooting.report-ad.description":"Побачили недоречну або образливу рекламу? Скопіюйте її дані, доки вона відображається, і надішліть у <support>підтримку Modrinth</support>. Ми перевіримо її та приберемо.",
"app.settings.troubleshooting.report-ad.no-ad.text":"Дочекайтеся завантаження реклами й повторіть спробу.",
"app.settings.troubleshooting.report-ad.no-ad.title":"Рекламу не знайдено",
"app.settings.troubleshooting.report-ad.title":"Повідомити про рекламу",
"app.shared-instance-publish.preparing":"Підготовка оновлення…",
"app.shared-instance-publish.pushed":"Оновлення надіслано",
"app.shared-instance-publish.uploading":"Надсилання оновлення…",
"instance.settings.defaults.open-app-settings":"Керувати типовими налаштуваннями",
"instance.settings.sidebar.label.game":"Гра",
"instance.settings.sidebar.label.instance":"Збірка",
"instance.settings.tabs.hooks.default-settings-description":"Для цієї збірки використовуються типові команди запуску, доки ви їх не зміните.",
"instance.settings.tabs.hooks.label":"Команди запуску",
"instance.settings.tabs.java.default-settings-description":"Для цієї збірки використовуються типові налаштування Java та пам’яті, доки ви їх не зміните.",
"instance.settings.tabs.java.label":"Java та пам’ять",
"instance.settings.tabs.syncing":"Синхронізація",
"instance.settings.tabs.window.default-settings-description":"Для цієї збірки використовуються типові налаштування вікна, доки ви їх не зміните.",
"instance.settings.tabs.window.label":"Вікно",
"instance.shared-instance.file-not-allowed.title":"Цей тип файлу заборонено",
"instance.shared-instance.publish-review.changes-description":"Перегляньте зміни, виявлені після останнього оновлення. Вони автоматично ввійдуть до публікації.",
"instance.shared-instance.publish-review.changes-tab":"Зміни",
"instance.shared-instance.publish-review.file-type-not-allowed":"Файли цього типу не можна надсилати.",
"instance.shared-instance.publish-review.files-description":"Ви також можете додати окремі файли. Виберіть додаткові файли для публікації.",
"instance.shared-instance.publish-review.files-tab":"Файли",
"instance.shared-instance.publish-review.storage-over-limit":"Перевищення ліміту на {size}. Зніміть вибір із частини файлів.",
"instance.shared-instance.publish-review.storage-title":"Ліміт сховища",
"instance.shared-instance.publish-review.storage-usage":"Використано {used} із {limit}"
}

UI = {
"color-picker.hex-value":"Шістнадцяткове значення: {label}",
"color-picker.hue":"Відтінок: {label}",
"color-picker.saturation-and-brightness":"Насиченість і яскравість: {label}",
"content.confirm-deletion.delete-button":"Видалити {count, number} {itemType}",
"content.page-layout.empty.hint":"Перегляньте або завантажте {contentType}, щоб почати",
"content.page-layout.search-placeholder":"Пошук серед {count, number} {contentType}…",
"content.selection-bar.bulk.deleting":"Видалення {progress}/{total}: {contentType}…",
"content.selection-bar.bulk.deleting-count":"Видалення {count, number}: {contentType}",
"content.selection-bar.bulk.deleting-waiting":"Видалення: {contentType}…",
"content.selection-bar.bulk.disabling":"Вимкнення {progress}/{total}: {contentType}…",
"content.selection-bar.bulk.disabling-count":"Вимкнення {count, number}: {contentType}",
"content.selection-bar.bulk.disabling-waiting":"Вимкнення: {contentType}…",
"content.selection-bar.bulk.enabling":"Увімкнення {progress}/{total}: {contentType}…",
"content.selection-bar.bulk.enabling-count":"Увімкнення {count, number}: {contentType}",
"content.selection-bar.bulk.enabling-waiting":"Увімкнення: {contentType}…",
"content.selection-bar.bulk.updating":"Оновлення {progress}/{total}: {contentType}…",
"content.selection-bar.bulk.updating-count":"Оновлення {count, number}: {contentType}",
"content.selection-bar.bulk.updating-waiting":"Оновлення: {contentType}…",
"content.selection-bar.selected-count":"Вибрано {count, number}: {contentType}",
"external-files.permissions-card.file-database-id":"Ідентифікатор у базі даних",
"hosting.save-banner.saved":"Збережено",
"profile.button.edit-restrictions":"Змінити обмеження",
"profile.button.edit-user":"Редагувати користувача",
"profile.button.restrict":"Обмежити користувача",
"profile.label.restricted":"Обмежений доступ",
"profile.restrict-user.admonition-body":"{username} залишиться в акаунті, але запити, що потребують вилучених дозволів, відхилятимуться до зняття обмежень.",
"profile.restrict-user.admonition-title":"Вибрані дозволи буде вилучено",
"profile.restrict-user.error-description":"Під час оновлення користувача виникла помилка. Спробуйте ще раз.",
"profile.restrict-user.header":"Обмежити користувача {username}",
"profile.restrict-user.private-reason-label":"Приватна примітка (лише для працівників)",
"profile.restrict-user.private-reason-placeholder":"Необов’язково",
"profile.restrict-user.reason-label":"Причина (видима користувачеві)",
"profile.restrict-user.reason-placeholder":"Необов’язково",
"profile.restrict-user.remove-button":"Зняти обмеження",
"profile.restrict-user.remove-error-title":"Не вдалося зняти обмеження",
"profile.restrict-user.remove-success-description":"Обмеження користувача {username} знято.",
"profile.restrict-user.remove-success-title":"Обмеження знято",
"profile.restrict-user.save-button":"Зберегти обмеження",
"profile.restrict-user.save-error-title":"Не вдалося зберегти обмеження",
"profile.restrict-user.save-success-description":"Обмеження користувача {username} оновлено.",
"profile.restrict-user.save-success-title":"Обмеження збережено",
"project.settings.environment.bulk.apply-button":"Застосувати до всіх версій",
"project.settings.environment.bulk.description":"Це замінить середовище для кожної версії цього проєкту.",
"project.settings.environment.bulk.title":"Застосувати середовище",
"project.settings.environment.modal.title":"Редагувати середовище проєкту",
"servers.backups.create.in-progress":"Створення резервної копії вже заплановано або триває",
"servers.backups.create.slots-full":"Усі {limit, plural, one {# слот ручних копій зайнятий} few {# слоти ручних копій зайняті} other {# слотів ручних копій зайнято}}",
"servers.backups.delete.error":"Не вдалося видалити {count, plural, one {резервну копію} few {# резервні копії} other {# резервних копій}}",
"servers.backups.history.auto-badge":"Автоматично",
"servers.backups.history.column.created":"Створено",
"servers.backups.history.column.created-by":"Автор",
"servers.backups.history.column.name":"Назва",
"servers.backups.history.creator-avatar-alt":"Аватар користувача {username}",
"servers.backups.history.creator.support":"Підтримка",
"servers.backups.history.more-options":"Додаткові дії для {name}",
"servers.backups.history.no-results":"За запитом резервних копій не знайдено.",
"servers.backups.history.rename":"Перейменувати",
"servers.backups.history.restore":"Відновити",
"servers.backups.history.search-label":"Пошук резервних копій",
"servers.backups.history.search-placeholder":"Шукати серед {count, plural, one {# резервної копії} few {# резервних копій} other {# резервних копій}}…",
"servers.backups.history.title":"Історія резервних копій",
"servers.backups.interval.days":"{days, plural, one {# день} few {# дні} other {# днів}}",
"servers.backups.interval.every-days":"Кожні {days, plural, one {# день} few {# дні} other {# днів}}",
"servers.backups.interval.every-hours":"Кожні {hours} год",
"servers.backups.interval.hours":"{hours} год",
"servers.backups.interval.weekly":"Щотижня",
"servers.backups.load-error.description":"Не вдалося завантажити резервні копії сервера.",
"servers.backups.load-error.title":"Помилка завантаження резервних копій",
"servers.backups.restore.operation-in-progress":"Операція з резервними копіями вже запланована або виконується",
"servers.backups.restore.server-running":"Перед відновленням зупиніть сервер",
"servers.backups.rotation-modal.admonition-body":"Зменшення кількості слотів означає, що {count, plural, one {найстарішу автоматичну копію} few {# найстаріші автоматичні копії} other {# найстаріших автоматичних копій}} буде безповоротно видалено. Цю дію не можна скасувати.",
"servers.backups.rotation-modal.admonition-header":"Попередження про видалення",
"servers.backups.rotation-modal.backups-label":"{count, plural, one {Резервна копія} few {Копії ({count})} other {Копії ({count})}}",
"servers.backups.rotation-modal.confirm":"Зберегти й видалити",
"servers.backups.rotation-modal.header":"Видалити автоматичні копії?",
"servers.backups.slots.auto.next":"Наступна: {time}",
"servers.backups.slots.auto.next-soon":"Незабаром почнеться резервне копіювання",
"servers.backups.slots.auto.off":"Вимкнено",
"servers.backups.slots.auto.off-description":"Увімкніть автоматичні копії в налаштуваннях",
"servers.backups.slots.auto.title":"Автоматичні копії",
"servers.backups.slots.limit":"/ {limit, plural, one {# слот} few {# слоти} other {# слотів}}",
"servers.backups.slots.manual.description":"Створені вручну",
"servers.backups.slots.manual.title":"Ручні резервні копії",
"servers.backups.toolbar.more-create-options":"Інші способи резервного копіювання",
"servers.region.out-of-stock":"(Немає в наявності)",
"servers.region.ping.failed":"Не вдалося виміряти затримку",
"servers.region.ping.latency":"{ping} мс",
"servers.region.ping.testing":"Перевірка з’єднання…",
"servers.settings.backups.auto.description":"Автоматично створювати резервні копії сервера через визначені проміжки часу.",
"servers.settings.backups.auto.title":"Автоматичні резервні копії",
"servers.settings.backups.interval.title":"Інтервал резервного копіювання",
"servers.settings.backups.manual-slots.description":"Визначте, скільки з {limit} слотів використовувати для ручних і автоматичних копій.",
"servers.settings.backups.manual-slots.title":"Слоти ручних копій",
"servers.settings.backups.save.error-title":"Не вдалося зберегти налаштування резервного копіювання",
"user-restrictions.group.collections":"Колекції",
"user-restrictions.group.create-projects":"Створення проєктів",
"user-restrictions.group.edit-projects":"Редагування проєктів",
"user-restrictions.group.organizations":"Організації",
"user-restrictions.group.profile":"Профіль",
"user-restrictions.group.versions":"Версії",
"user-restrictions.scope.collection-create":"Створювати колекції",
"user-restrictions.scope.collection-delete":"Видаляти колекції",
"user-restrictions.scope.collection-write":"Редагувати колекції",
"user-restrictions.scope.organization-create":"Створювати організації",
"user-restrictions.scope.organization-delete":"Видаляти організації",
"user-restrictions.scope.organization-write":"Редагувати організації",
"user-restrictions.scope.project-create":"Створювати нові проєкти",
"user-restrictions.scope.project-delete":"Видаляти проєкти",
"user-restrictions.scope.project-write":"Редагувати проєкти, керувати командою й приймати запрошення",
"user-restrictions.scope.user-write":"Редагувати профіль, відстежувати проєкти та змінювати налаштування",
"user-restrictions.scope.version-create":"Завантажувати нові версії",
"user-restrictions.scope.version-delete":"Видаляти версії та їхні файли",
"user-restrictions.scope.version-write":"Редагувати версії та додавати файли"
}
NEW = {
"fre-launch.welcome-screen.title":("Вітаємо у FRE LAUNCH","Welcome to FRE LAUNCH"),
"fre-launch.offline.title":("Локальний нік","Local nickname"),
"fre-launch.offline.placeholder":("Введи нік","Your nickname"),
"fre-launch.offline.create":("Створити","Create"),
"fre-launch.offline.short":("Локальний","Local"),
"fre-launch.offline.disclaimer":("Для одиночної гри та серверів без перевірки ліцензії. Це не Microsoft-акаунт.","For single-player and offline-mode servers only. Not a Microsoft account.")
}

def _complete(english, ukrainian, curated, field):
    unexpected=set(curated)-set(english)
    if unexpected:
        raise RuntimeError(f"Translation key mismatch: {sorted(unexpected)}")
    added=0
    english_fallback=0
    for key, entry in english.items():
        if key in ukrainian and str(ukrainian[key].get(field,"")).strip():
            continue
        if key in curated:
            ukrainian[key]={field:curated[key]}
            added+=1
        else:
            ukrainian[key]={field:entry[field]}
            english_fallback+=1
    return added, english_fallback

def apply(root):
    app_uk=root/"apps/app-frontend/src/locales/uk-UA/index.json"
    app_en=root/"apps/app-frontend/src/locales/en-US/index.json"
    ui_uk=root/"packages/ui/src/locales/uk-UA/index.json"
    ui_en=root/"packages/ui/src/locales/en-US/index.json"
    ea=json.loads(app_en.read_text(encoding="utf-8"))
    ua=json.loads(app_uk.read_text(encoding="utf-8"))
    eu=json.loads(ui_en.read_text(encoding="utf-8"))
    uu=json.loads(ui_uk.read_text(encoding="utf-8"))
    app,_a=_complete(ea,ua,APP,"message")
    ui,_u=_complete(eu,uu,UI,"defaultMessage")
    for key,(uk,en) in NEW.items():
        if key in ua or key in ea:
            raise RuntimeError(f"Locale name collision: {key}")
        ua[key]={"message":uk}
        ea[key]={"message":en}
    # A nickname-only profile completes onboarding without a Microsoft login;
    # name this as choosing a profile, not signing into official services.
    ea["onboarding-checklist.login-minecraft"]={"message":"Set up Minecraft profile"}
    ua["onboarding-checklist.login-minecraft"]={"message":"Налаштувати профіль Minecraft"}
    # App settings must show the fork name rather than the upstream trademark.
    for catalog in (ea, ua):
        if "app.settings.app-version" in catalog:
            catalog["app.settings.app-version"]["message"] = "FRE LAUNCH {version}"
    for path, content in [(app_en,ea),(app_uk,ua),(ui_uk,uu)]:
        path.write_text(json.dumps(content,ensure_ascii=False,indent=2,sort_keys=True)+"\n",encoding="utf-8")
    print(f"Ukrainian translations completed: app={app}, UI={ui}, new profiles={len(NEW)}. "
          f"English-only fallback strings app={_a}, UI={_u}.")
