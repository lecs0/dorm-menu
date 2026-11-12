package hu.dormmenu

import android.app.*
import android.os.*
import android.content.*
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.content.res.Configuration
import android.view.View
import android.view.Gravity
import android.net.Uri
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.text.InputType
import android.text.TextWatcher
import android.text.Editable
import android.text.SpannableString
import android.text.SpannableStringBuilder
import android.text.style.ForegroundColorSpan
import android.text.style.BackgroundColorSpan
import android.text.style.StyleSpan
import android.text.style.RelativeSizeSpan
import android.widget.*
import org.json.*
import java.io.File
import java.net.URL
import java.net.HttpURLConnection
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec
import android.util.Base64
import java.time.*
import java.util.concurrent.Executors

class MainActivity : Activity() {
    companion object { init { System.loadLibrary("dorm_app") } }
    external fun process(input: String): String
    private val executor = Executors.newSingleThreadExecutor()
    private val prefs by lazy { getSharedPreferences("settings", MODE_PRIVATE) }
    private lateinit var root: LinearLayout
    private var status = "Mentett menü • frissítésre vár"
    private var screen = "menu"
    private var lastDay = ""
    private val dark get() = when (prefs.getString("theme", "system")) {
        "light" -> false
        "dark" -> true
        else -> resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
    }
    private fun color(light: String, night: String) = Color.parseColor(if (dark) night else light)
    private val ink get() = color("#17231D", "#F2F3F2")
    private val green get() = color("#21583C", "#B5CEB9")
    private val surface get() = color("#FFFFFF", "#303133")
    private val canvasColor get() = color("#F4F5F2", "#1C1D1F")
    private val accent get() = color("#F9E5D2", "#53443D")
    private val highlightNames = arrayOf("Barack", "Zsálya", "Levendula", "Homok", "Rózsa", "Égkék", "Menta", "Borostyán")
    private fun highlightColor(index: Int): Int {
        val light = arrayOf("#F9E5D2", "#DDECE1", "#EAE4F3", "#F2E9CE", "#F7DFE5", "#DDEAF6", "#D8EEE9", "#F6E5BB")
        val night = arrayOf("#53443D", "#3A4A40", "#494250", "#4D493D", "#523D46", "#374957", "#344C49", "#514630")
        val i = index.coerceIn(0, light.lastIndex)
        return color(light[i], night[i])
    }
    private fun dp(n: Int) = (n * resources.displayMetrics.density).toInt()
    private fun shape(c: Int) = GradientDrawable().apply { setColor(c); cornerRadius = dp(22).toFloat() }
    private var syncError: String? = null
    private val dismissedNotices = mutableSetOf<String>()
    private var lastOnline = false
    private var networkCallback: android.net.ConnectivityManager.NetworkCallback? = null
    private fun online(): Boolean {
        val manager = getSystemService(android.net.ConnectivityManager::class.java)
        val capabilities = manager.getNetworkCapabilities(manager.activeNetwork)
        return capabilities?.hasCapability(android.net.NetworkCapabilities.NET_CAPABILITY_INTERNET) == true
    }
    private fun watchNetwork() {
        val manager = getSystemService(android.net.ConnectivityManager::class.java)
        lastOnline = online()
        val callback = object : android.net.ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: android.net.Network) = networkChanged()
            override fun onLost(network: android.net.Network) = networkChanged()
            override fun onCapabilitiesChanged(network: android.net.Network, capabilities: android.net.NetworkCapabilities) = networkChanged()
        }
        try { manager.registerDefaultNetworkCallback(callback); networkCallback = callback } catch (_: Exception) { }
    }
    private fun networkChanged() {
        Handler(Looper.getMainLooper()).postDelayed({
            if (isDestroyed) return@postDelayed
            val now = online()
            if (now != lastOnline) {
                lastOnline = now; dismissedNotices.remove("offline")
                if (screen == "menu") { render(); if (now) autoRefresh() }
            }
        }, 250)
    }
    private var syncing = false
    private var selectedMenu = ""
    private var selectedDay = ""
    private val zone = ZoneId.of("Europe/Budapest")
    private fun today() = LocalDate.now(zone).toString()
    private fun formatTime(value: String): String = try { java.time.format.DateTimeFormatter.ofPattern("yyyy.MM.dd HH:mm").withZone(zone).format(Instant.parse(value)) + " (Budapest)" } catch (_: Exception) { "Nem ismert" }
    private fun statusLabel(value: String) = when (value) { "current" -> "Aktuális"; "upcoming" -> "Következő"; "expired" -> "Lejárt"; else -> "Ismeretlen dátum" }
    private fun allowedEndpoint(value: String): Boolean = try {
        val url = URL(value)
        val transport = url.protocol == "https" || (BuildConfig.DEBUG && url.protocol == "http" && url.host == "127.0.0.1")
        transport && url.userInfo == null && url.query == null && url.ref == null
    } catch (_: Exception) { false }
    private fun cache() = File(filesDir, "menus.json")
    private fun preferences() = JSONObject().put("keywords", JSONArray(prefs.getString("keywords", "[]")))
        .put("reminders", prefs.getBoolean("reminders", false)).put("reminder_hour", prefs.getInt("hour", 18)).put("reminder_minute", prefs.getInt("minute", 0))
    override fun onCreate(savedInstanceState: Bundle?) {
        when (getSharedPreferences("settings", MODE_PRIVATE).getString("theme", "system")) {
            "light" -> setTheme(R.style.AppThemeExplicitLight)
            "dark" -> setTheme(R.style.AppThemeExplicitDark)
            else -> setTheme(R.style.AppTheme)
        }
        super.onCreate(savedInstanceState)
        window.decorView.layoutDirection = View.LAYOUT_DIRECTION_LTR
        watchNetwork()
        if (savedInstanceState?.getString("screen") == "settings") settings() else render()
    }
    override fun onSaveInstanceState(outState: Bundle) {
        outState.putString("screen", screen)
        super.onSaveInstanceState(outState)
    }
    override fun onResume() { super.onResume(); if (::root.isInitialized) { if (lastDay != today()) { selectedMenu = ""; selectedDay = ""; lastDay = today() }; if (screen == "menu") { render(); autoRefresh() } } }
    override fun onDestroy() { networkCallback?.let { try { getSystemService(android.net.ConnectivityManager::class.java).unregisterNetworkCallback(it) } catch (_: Exception) { } }; super.onDestroy(); executor.shutdown() }
    private fun text(s: String, size: Float = 16f, warning: Boolean = false) {
        root.addView(TextView(this).apply { text = s; textSize = size; setTextColor(ink); setPadding(dp(8), dp(8), dp(8), dp(8)); if (size >= 20) setTypeface(null, Typeface.BOLD); if (warning) background = shape(accent) })
    }
    private fun button(label: String, action: () -> Unit) {
        root.addView(Button(this).apply { text = label; isAllCaps = false; textSize = 16f; minHeight = dp(48); setTextColor(green); backgroundTintList = android.content.res.ColorStateList.valueOf(surface); setOnClickListener { action() } })
    }
    private fun notice(key: String, title: String, detail: String, actionLabel: String? = null, action: (() -> Unit)? = null) {
        if (key in dismissedNotices) return
        val row = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.TOP; setPadding(dp(12), dp(10), dp(8), dp(10)); background = shape(color("#F7EBDD", "#443C35")) }
        val body = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        body.addView(TextView(this).apply { text = title; textSize = 16f; setTextColor(ink); setTypeface(null, Typeface.BOLD) })
        body.addView(TextView(this).apply { text = detail; textSize = 14f; setTextColor(ink); setPadding(0, dp(4), 0, 0) })
        if (actionLabel != null && action != null) body.addView(Button(this).apply {
            text = actionLabel; isAllCaps = false; textSize = 14f; minHeight = dp(48); setTextColor(green)
            backgroundTintList = android.content.res.ColorStateList.valueOf(surface); setOnClickListener { action() }
        })
        row.addView(body, LinearLayout.LayoutParams(0, -2, 1f))
        row.addView(Button(this).apply {
            text = "×"; textSize = 22f; isAllCaps = false; minWidth = 0; minimumWidth = dp(48); minHeight = dp(48)
            setPadding(0, 0, 0, 0); setTextColor(ink); background = shape(color("#F7EBDD", "#443C35")); contentDescription = "Értesítés bezárása: $title"
            setOnClickListener { dismissedNotices.add(key); render() }
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
        root.addView(row, LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(12) })
    }
    private fun card(highlight: Boolean = false, cardColor: Int? = null, content: () -> Unit) {
        val parent = root
        val child = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(dp(12), dp(12), dp(12), dp(12)); background = shape(cardColor ?: if (highlight) accent else surface) }
        parent.addView(child, LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(14) })
        root = child; content(); root = parent
    }
    private fun frame() {
        val outer = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setBackgroundColor(canvasColor) }
        outer.setOnApplyWindowInsetsListener { v, insets -> v.setPadding(insets.systemWindowInsetLeft, insets.systemWindowInsetTop, insets.systemWindowInsetRight, insets.systemWindowInsetBottom); insets }
        root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; isFocusableInTouchMode = true; setPadding(dp(16), dp(12), dp(16), dp(20)) }
        outer.addView(ScrollView(this).apply { isFillViewport = true; addView(root) }, LinearLayout.LayoutParams(-1, 0, 1f))
        val header = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL; gravity = Gravity.CENTER_VERTICAL }
        if (screen != "menu") {
            header.addView(Button(this).apply {
                text = "‹"; textSize = 26f; minWidth = 0; minimumWidth = dp(48); minHeight = dp(48); setPadding(0, 0, 0, 0)
                isAllCaps = false; setTextColor(green); background = shape(surface); contentDescription = "Vissza"
                setOnClickListener { navigateBack() }
            }, LinearLayout.LayoutParams(dp(48), dp(48)).apply { marginEnd = dp(12) })
        }
        header.addView(TextView(this).apply {
            text = when (screen) { "menu" -> "Kollégiumi menü"; "connection" -> "Kapcsolat"; "source" -> "Étlap és forrás"; else -> "Beállítások" }
            textSize = 24f; setTextColor(ink); setTypeface(null, Typeface.BOLD); setPadding(dp(4), dp(8), dp(4), dp(8))
        }, LinearLayout.LayoutParams(0, -2, 1f))
        if (screen == "menu") {
            header.addView(Button(this).apply {
                text = "⚙"; textSize = 24f; minWidth = 0; minimumWidth = dp(48); minHeight = dp(48); setPadding(0, 0, 0, 0)
                isAllCaps = false; setTextColor(green); background = shape(surface); contentDescription = "Beállítások"
                setOnClickListener { settings() }
            }, LinearLayout.LayoutParams(dp(48), dp(48)))
        }
        root.addView(header, LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(12) })
        setContentView(outer)
    }
    private fun navigateBack() {
        when (screen) {
            "connection" -> AlertDialog.Builder(this).setTitle("Visszalépsz mentés nélkül?").setMessage("A kapcsolat módosításait külön kell menteni.").setNegativeButton("Maradok", null).setPositiveButton("Vissza") { _, _ -> settings() }.show()
            "source" -> settings()
            else -> render()
        }
    }
    @Deprecated("Platform back navigation") override fun onBackPressed() { if (screen != "menu") navigateBack() else super.onBackPressed() }
    private fun updateReminders() {
        try { val data = model(); if (data != null) schedule(data) else cancelReminders(this) } catch (_: Exception) { cancelReminders(this) }
    }
    private fun watch(field: EditText, changed: (String) -> Unit) {
        field.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) { changed(s?.toString() ?: "") }
            override fun afterTextChanged(s: Editable?) {}
        })
    }
    private fun model(): JSONObject? {
        if (!cache().exists()) return null
        val result = JSONObject(process(JSONObject().put("today", today()).put("cache", cache().readText()).put("preferences", preferences()).toString()))
        if (result.has("error")) throw IllegalStateException(result.getString("error"))
        return result
    }
    private fun render() {
        screen = "menu"; frame()
        val isOnline = online()
        if (!isOnline) notice("offline", "Nincs internetkapcsolat", if (cache().exists()) "A mentett étlapot látod. A frissítéshez kapcsolódj az internethez." else "Kapcsolódj az internethez az étlap letöltéséhez.")
        if (isOnline && syncError != null && !syncing) notice("sync", "Nem sikerült a frissítés", syncError!!, "Újrapróbálom") { refresh(true) }
        try {
            val data = model() ?: run { card { text("Még nincs mentett étlap", 22f); text("A kapcsolódáshoz add meg a hozzáférési tokent a beállításokban."); button("Kapcsolat beállítása") { settings(true) }; button("Frissítés ↻") { refresh(true) } }; return }
            when (data.optString("source_status")) {
                "error" -> notice("source", "A forrás frissítése sikertelen", "A szerver az utoljára mentett étlapot küldi. Ellenőrizd a forrás frissítését később.")
                "unavailable" -> notice("source", "A forrás még nem elérhető", "A szerver még nem tudott sikeresen étlapot letölteni.")
                else -> dismissedNotices.remove("source")
            }
            if (isOnline && syncError == null && !syncing && System.currentTimeMillis() - prefs.getLong("last_sync", 0) > prefs.getLong("interval", 21600) * 1000) notice("stale", "Régebbi mentett étlap", "Frissíts, hogy lásd a legújabb adatokat.", "Frissítés") { refresh(true) }
            val menus = data.getJSONArray("menus")
            val defaultMenu = data.optJSONObject("current_menu") ?: data.optJSONObject("next_menu")
            val desired = selectedMenu.ifEmpty { defaultMenu?.optString("menu_id") ?: "" }
            val index = (0 until menus.length()).firstOrNull { menus.getJSONObject(it).getString("menu_id") == desired } ?: (menus.length() - 1)
            val m = if (index >= 0) menus.getJSONObject(index) else null
            val missing = data.getJSONArray("unpublished")
            val labels = (0 until menus.length()).map { menus.getJSONObject(it).getJSONObject("entry").getString("label") } + (0 until missing.length()).map { missing.getJSONObject(it).getString("label") + " • még nincs közzétéve" }
            if (m != null) {
                selectedMenu = m.getString("menu_id")
                if (m.getString("status") != "current") notice("menu_" + m.getString("menu_id") + m.getString("status"), statusLabel(m.getString("status")) + " étlap", "Érvényesség: " + m.getJSONObject("entry").optString("valid_from") + " – " + m.getJSONObject("entry").optString("valid_until"))
                val days = m.getJSONArray("days")
                val dayIndex = (0 until days.length()).firstOrNull { days.getJSONObject(it).getString("date") == selectedDay } ?: (0 until days.length()).firstOrNull { days.getJSONObject(it).getString("date") == today() } ?: 0
                val strip = LinearLayout(this).apply { gravity = Gravity.CENTER }
                for (i in 0 until days.length()) {
                    val day = days.getJSONObject(i); val date = LocalDate.parse(day.getString("date")); val isToday = date.toString() == today()
                    val selected = i == dayIndex
                    val tile = LinearLayout(this).apply {
                        orientation = LinearLayout.VERTICAL
                        gravity = Gravity.CENTER
                        minimumHeight = dp(72)
                        setPadding(dp(4), dp(8), dp(4), dp(8))
                        background = shape(if (selected) green else surface)
                        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                    }
                    val dayColor = if (selected) canvasColor else ink
                    val shortName = listOf("H", "K", "Sze", "Cs", "P", "Szo", "V")[date.dayOfWeek.value - 1]
                    tile.addView(TextView(this).apply {
                        text = shortName; textSize = 14f; gravity = Gravity.CENTER
                        setTextColor(dayColor); setTypeface(null, Typeface.BOLD)
                        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                    })
                    tile.addView(TextView(this).apply {
                        text = date.dayOfMonth.toString(); textSize = 20f; gravity = Gravity.CENTER
                        setTextColor(dayColor); setTypeface(null, Typeface.BOLD)
                        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                    })
                    val item = LinearLayout(this).apply {
                        orientation = LinearLayout.VERTICAL; gravity = Gravity.CENTER_HORIZONTAL
                        isClickable = true; isFocusable = true; isSelected = selected
                        contentDescription = day.getString("day_name") + " " + date + (if (isToday) ", ma" else "") + (if (selected) ", kiválasztva" else "")
                        setOnClickListener { selectedDay = date.toString(); render() }
                    }
                    item.addView(tile, LinearLayout.LayoutParams(-1, -2))
                    item.addView(TextView(this).apply {
                        text = if (isToday) "MA" else " "; textSize = 11f; gravity = Gravity.CENTER
                        setTextColor(green); setTypeface(null, Typeface.BOLD); setPadding(0, dp(3), 0, 0)
                        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                    }, LinearLayout.LayoutParams(-1, -2))
                    strip.addView(item, LinearLayout.LayoutParams(dp(58), -2).apply { marginStart = dp(4); marginEnd = dp(4) })
                }
                root.addView(HorizontalScrollView(this).apply { isHorizontalScrollBarEnabled = false; isFillViewport = true; addView(strip); post { smoothScrollTo(dp(dayIndex * 66 - 120).coerceAtLeast(0), 0) } })
                root.addView(View(this), LinearLayout.LayoutParams(1, dp(28)))
                if (days.length() > 0) {
                    val day = days.getJSONObject(dayIndex); selectedDay = day.getString("date")
                    val meals = day.getJSONArray("meals")
                    for ((category, title) in listOf("breakfast" to "Reggeli", "lunch" to "Ebéd", "dinner" to "Vacsora")) {
                        if (!prefs.getBoolean("show_$category", true)) continue
                        val meal = (0 until meals.length()).map { meals.getJSONObject(it) }.firstOrNull { it.getString("category") == category }
                        val hits = meal?.optJSONArray("keyword_matches")
                        val codes = meal?.optJSONArray("source_allergen_codes")
                        val selectedCodes = prefs.getStringSet("allergen_codes", emptySet()) ?: emptySet()
                        val matches = (0 until (codes?.length() ?: 0)).flatMap { codes!!.getString(it).split(Regex("[^0-9]+")) }.filter { it.isNotEmpty() && it in selectedCodes }.distinct()
                        val keywordHit = hits != null && hits.length() > 0
                        val allergenLineMode = prefs.getBoolean("allergen_line_mode", false)
                        val keywordColor = highlightColor(prefs.getInt("keyword_color", 0))
                        val allergenColor = highlightColor(prefs.getInt("allergen_color", 1))
                        val lineCodeMatches = meal?.optString("original_text")?.lines()?.any { line ->
                            Regex("(?<![\\p{L}\\d])\\d+(?:[ \\t]*,[ \\t]*\\d+)*,?").findAll(line).any { group -> group.value.split(Regex("[^0-9]+" )).any { it in selectedCodes } }
                        } ?: false
                        val cardColor = if (keywordHit) keywordColor else if (matches.isNotEmpty() && (!allergenLineMode || !lineCodeMatches)) allergenColor else surface
                        card(cardColor = cardColor) {
                            text(title, 22f)
                            root.addView(View(this).apply { setBackgroundColor(green); alpha = 0.35f }, LinearLayout.LayoutParams(-1, dp(1)).apply { setMargins(dp(8), dp(2), dp(8), dp(8)) })
                            if (meal == null) text("Ehhez az étkezéshez nem szerepel menü a forrásban.") else {
                                val original = meal.getString("original_text")
                                val formatted = SpannableString(original)
                                if (matches.isNotEmpty() && allergenLineMode) {
                                    // PDF extraction preserves visual lines, not reliable dish boundaries.
                                    // Mark the code and its immediately preceding text line when it is alone.
                                    val lines = original.split("\n")
                                    var offset = 0
                                    val offsets = lines.map { line -> val start = offset; offset += line.length + 1; start }
                                    val numericOnly = Regex("^[\\s\\d,;]+$")
                                    val groups = Regex("(?<![\\p{L}\\d])\\d+(?:[ \\t]*,[ \\t]*\\d+)*,?")
                                    for ((i, line) in lines.withIndex()) {
                                        val matched = groups.findAll(line).any { group -> group.value.split(Regex("[^0-9]+" )).any { it in selectedCodes } }
                                        if (!matched) continue
                                        val first = if (i > 0 && numericOnly.matches(line) && !numericOnly.matches(lines[i - 1])) i - 1 else i
                                        val start = offsets[first]; val end = offsets[i] + line.length
                                        if (start < end) { formatted.setSpan(BackgroundColorSpan(allergenColor), start, end, 0); formatted.setSpan(StyleSpan(Typeface.BOLD), start, end, 0) }
                                    }
                                }
                                Regex("(?<![\\p{L}\\d])\\d+(?:\\s*,\\s*\\d+)*,?").findAll(original).forEach { match ->
                                    formatted.setSpan(ForegroundColorSpan(green), match.range.first, match.range.last + 1, 0)
                                    formatted.setSpan(RelativeSizeSpan(0.85f), match.range.first, match.range.last + 1, 0)
                                }
                                val displayed = SpannableStringBuilder(formatted)
                                if (!prefs.getBoolean("show_inline_allergen_codes", true) && codes != null) {
                                    // Only remove line-ending groups that match published code groups.
                                    val published = (0 until codes.length()).map { index -> Regex("\\d+").findAll(codes.getString(index)).map { it.value }.toList() }.toSet()
                                    val codeAtEnd = Regex("(?<![\\p{L}\\d])\\d+(?:[ \\t]*,[ \\t]*\\d+)*,?[ \\t]*(?=\\n|$)")
                                    for (match in codeAtEnd.findAll(original).toList().asReversed()) {
                                        val group = Regex("\\d+").findAll(match.value).map { it.value }.toList()
                                        if (group !in published) continue
                                        var start = match.range.first
                                        var end = match.range.last + 1
                                        val lineStart = original.lastIndexOf('\n', start - 1) + 1
                                        if (original.substring(lineStart, start).isBlank()) {
                                            start = lineStart
                                            if (end < original.length && original[end] == '\n') end++
                                            else if (start > 0 && original[start - 1] == '\n') start--
                                        } else {
                                            while (start > lineStart && original[start - 1] in " \t") start--
                                        }
                                        displayed.delete(start, end)
                                    }
                                }
                                root.addView(TextView(this).apply { text = displayed; textSize = 18f; setTextColor(ink); setPadding(dp(8), dp(8), dp(8), dp(8)); contentDescription = displayed.toString() })
                                if (hits != null && hits.length() > 0) text("Figyelt kulcsszó: " + (0 until hits.length()).joinToString(", ") { hits.getString(it) }, 14f)
                                text(if (codes == null || codes.length() == 0) "Allergénkódok: nincs közölt adat" else "Allergénkódok: " + (0 until codes.length()).joinToString("; ") { codes.getString(it) }, 13f)
                            }
                        }
                    }
                if (listOf("breakfast", "lunch", "dinner").none { prefs.getBoolean("show_$it", true) }) text("Minden étkezés el van rejtve. A beállításokban újra megjelenítheted őket.", 16f)
                } else text("Ehhez a héthez még nincs napi menü.")
            }
            button((m?.getJSONObject("entry")?.getString("label") ?: "Elérhető hetek") + "  ▾") {
                AlertDialog.Builder(this).setTitle("Hét kiválasztása").setItems(labels.toTypedArray()) { _, pos ->
                    if (pos < menus.length()) { selectedMenu = menus.getJSONObject(pos).getString("menu_id"); selectedDay = ""; render() }
                    else AlertDialog.Builder(this).setTitle("Még nincs közzétéve").setMessage("Ehhez a héthez még nem érkezett étlap. Nézz vissza később!").setPositiveButton("Rendben", null).show()
                }.show()
            }
            button(if (syncing) "Frissítés folyamatban…" else "Frissítés ↻") { refresh(true) }
            text("Forrás frissítve: " + formatTime(data.optString("last_successful_update")), 13f)
            if (syncing) text("Frissítés folyamatban…", 13f)
            schedule(data)
        } catch (_: Exception) { text("A mentett étlap nem olvasható. Próbáld meg frissíteni.", warning = true) }
    }
    private fun autoRefresh() {
        val interval = prefs.getLong("interval", 21600).coerceAtLeast(900) * 1000
        if (System.currentTimeMillis() - prefs.getLong("last_sync", 0) >= interval && System.currentTimeMillis() >= prefs.getLong("retry_after", 0)) refresh(false)
    }
    private fun refresh(manual: Boolean) {
        if (syncing) return
        val endpoint = prefs.getString("endpoint", "https://koliapp.lecso.me")!!.trimEnd('/')
        val token = try { readToken() } catch (_: Exception) { "" }
        if (!allowedEndpoint(endpoint) || token.isEmpty()) { syncError = "A kapcsolathoz adj meg érvényes szervercímet és tokent a beállításokban."; dismissedNotices.remove("sync"); render(); return }
        syncing = true; syncError = null; dismissedNotices.remove("sync"); render()
        executor.execute {
            var connection: HttpURLConnection? = null
            try {
                val conn = URL("$endpoint/api/v1/menus").openConnection() as HttpURLConnection
                connection = conn
                conn.instanceFollowRedirects = false; conn.connectTimeout = 10000; conn.readTimeout = 15000
                conn.setRequestProperty("Authorization", "Bearer $token")
                if (cache().exists()) prefs.getString("etag", null)?.let { conn.setRequestProperty("If-None-Match", it) }
                when (conn.responseCode) {
                    200 -> {
                        val bytes = conn.inputStream.use { input ->
                            val out = java.io.ByteArrayOutputStream(); val buffer = ByteArray(8192)
                            while (true) { val n = input.read(buffer); if (n < 0) break; require(out.size() + n <= 4_000_000) { "Túl nagy válasz érkezett" }; out.write(buffer, 0, n) }; out.toByteArray()
                        }
                        require(bytes.size <= 4_000_000) { "Túl nagy válasz érkezett" }
                        val json = String(bytes, Charsets.UTF_8)
                        val validated = JSONObject(process(JSONObject().put("today", today()).put("cache", json).put("preferences", preferences()).toString()))
                        require(!validated.has("error")) { validated.optString("error") }
                        val tmp = File(filesDir, "menus.tmp"); tmp.outputStream().use { it.write(bytes); it.fd.sync() }
                        check(tmp.renameTo(cache())) { "Az étlap mentése sikertelen" }
                        prefs.edit().putString("etag", conn.getHeaderField("ETag")).apply()
                    }
                    304 -> require(cache().exists()) { "Nincs mentett étlap; frissíts újra" }
                    401 -> throw IllegalStateException("A token érvénytelen vagy visszavonták. Cseréld ki a Kapcsolat beállításaiban.")
                    429 -> throw IllegalStateException("Túl sok kérés. Próbáld újra később.")
                    else -> throw IllegalStateException("Szerverhiba (${conn.responseCode})")
                }
                prefs.edit().putLong("last_sync", System.currentTimeMillis()).putInt("failures", 0).putLong("retry_after", 0).apply()
                status = "Frissítve · ${formatTime(Instant.now().toString())}"
                syncError = null
            } catch (e: Exception) {
                val failures = (prefs.getInt("failures", 0) + 1).coerceAtMost(8)
                prefs.edit().putInt("failures", failures).putLong("retry_after", System.currentTimeMillis() + (60000L shl failures)).apply()
                syncError = if (e is IllegalStateException) (e.message ?: "Próbáld újra később.") else "A szerver nem érhető el. A mentett étlap megmaradt."
            } finally { connection?.disconnect() }
            runOnUiThread { syncing = false; if (syncError == null) dismissedNotices.remove("stale"); if (!isDestroyed && screen == "menu") render() }
        }
    }
    private fun settings(connection: Boolean = false) {
        screen = if (connection) "connection" else "settings"; frame()
        fun field(label: String, value: String, secret: Boolean = false): EditText {
            text(label, 14f)
            return EditText(this).apply { setText(value); textSize = 17f; setTextColor(ink); minHeight = dp(56); setPadding(dp(12), dp(8), dp(12), dp(8)); backgroundTintList = android.content.res.ColorStateList.valueOf(green); contentDescription = label; if (secret) { inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD; importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO; isSaveEnabled = false }; root.addView(this) }
        }
        if (connection) {
            text("A hozzáférési token titkosítva, ezen a telefonon tárolódik.")
            val endpoint = field("Szerver címe", prefs.getString("endpoint", "https://koliapp.lecso.me")!!)
            val token = field("Új token • üresen hagyva megmarad a mentett", "", true)
            text(if (prefs.contains("token")) "Token biztonságosan mentve" else "Még nincs mentett token", 14f)
            if (prefs.contains("token")) {
                button("Mentett token megjelenítése") {
                    try {
                        val saved = readToken()
                        if (saved.isEmpty()) throw IllegalStateException()
                        val value = TextView(this).apply {
                            text = saved; textSize = 16f; setTextColor(ink); setTextIsSelectable(true)
                            setPadding(dp(24), dp(12), dp(24), dp(12))
                        }
                        val dialog = AlertDialog.Builder(this).setTitle("Mentett token")
                            .setMessage("A token titkos hozzáférési kulcs. Hosszan nyomva kijelölheted és másolhatod.")
                            .setView(value).setPositiveButton("Bezárás", null).create()
                        dialog.window?.setFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE, android.view.WindowManager.LayoutParams.FLAG_SECURE)
                        dialog.show()
                        dialog.window?.setFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE, android.view.WindowManager.LayoutParams.FLAG_SECURE)
                    } catch (_: Exception) { Toast.makeText(this, "A mentett token nem olvasható ezen a készüléken.", Toast.LENGTH_LONG).show() }
                }
            }
            button("Kapcsolat mentése") {
                val address = endpoint.text.toString().trim().trimEnd('/')
                if (!allowedEndpoint(address)) { endpoint.error = "Adj meg érvényes HTTPS-címet."; return@button }
                try {
                    if (token.text.isNotBlank()) saveToken(token.text.toString().trim())
                    val edit = prefs.edit().putString("endpoint", address).putLong("last_sync", 0).putLong("retry_after", 0)
                    if (address != prefs.getString("endpoint", "https://koliapp.lecso.me")) { cache().delete(); edit.remove("etag"); cancelReminders(this); selectedMenu = ""; selectedDay = "" }
                    edit.apply(); syncError = null; dismissedNotices.remove("sync"); status = "Kapcsolat mentve • frissítsd az étlapot"; render()
                } catch (_: Exception) { text("A token mentése sikertelen. Próbáld újra.", warning = true) }
            }
            button("Vissza a beállításokhoz") { navigateBack() }; return
        }
        card {
            text("Megjelenés", 22f)
            val labels = arrayOf("Rendszer szerint", "Világos", "Sötét")
            val values = arrayOf("system", "light", "dark")
            val selected = values.indexOf(prefs.getString("theme", "system")).coerceAtLeast(0)
            button("Téma: " + labels[selected] + "  ▾") {
                AlertDialog.Builder(this).setTitle("Téma").setSingleChoiceItems(labels, selected) { dialog, which ->
                    dialog.dismiss()
                    if (which != selected) {
                        prefs.edit().putString("theme", values[which]).apply()
                        recreate()
                    }
                }.setNegativeButton("Mégse", null).show()
            }
            text("A rendszer szerinti téma a telefon beállítását követi.", 14f)
        }
        lateinit var keywords: EditText
        card { text("Figyelt ételek", 22f); keywords = field("Kulcsszavak, vesszővel elválasztva", JSONArray(prefs.getString("keywords", "[]")).let { a -> (0 until a.length()).joinToString(", ") { a.getString(it) } }); text("A kis- és nagybetű nem számít, az ékezet igen. A „hal” a „halászlé” szóban is találat. Törléshez vedd ki a szót a listából.", 14f) }
        text("A módosításokat automatikusan mentjük.", 14f)
        watch(keywords) { value ->
            prefs.edit().putString("keywords", JSONArray(value.split(',').map { it.trim() }.filter { it.isNotEmpty() }.distinct()).toString()).apply()
            updateReminders()
        }
        val allergenSelection = (prefs.getStringSet("allergen_codes", emptySet()) ?: emptySet()).toMutableSet()
        card {
            text("Figyelt allergénkódok", 22f)
            text("A forrás kódjainak jelentése és teljessége nem ellenőrzött. Allergia esetén egyeztess a kollégiummal.", 14f)
            val choose = Button(this).apply { isAllCaps = false; minHeight = dp(48); setTextColor(green); backgroundTintList = android.content.res.ColorStateList.valueOf(surface) }
            fun updateLabel() { choose.text = if (allergenSelection.isEmpty()) "Kódok kiválasztása" else "Figyelt kódok: " + allergenSelection.sorted().joinToString(", ") }
            updateLabel()
            choose.setOnClickListener {
                val draft = allergenSelection.toMutableSet()
                val choices = (1..7).map { "$it. allergénkód" }.toTypedArray()
                val checked = BooleanArray(7) { (it + 1).toString() in draft }
                AlertDialog.Builder(this).setTitle("Figyelt allergénkódok").setMultiChoiceItems(choices, checked) { _, which, enabled -> if (enabled) draft.add((which + 1).toString()) else draft.remove((which + 1).toString()) }.setNegativeButton("Mégse", null).setPositiveButton("Kész") { _, _ -> allergenSelection.clear(); allergenSelection.addAll(draft); prefs.edit().putStringSet("allergen_codes", allergenSelection.toSet()).apply(); updateLabel() }.show()
            }
            root.addView(choose)
        }
        var keywordColorChoice = prefs.getInt("keyword_color", 0).coerceIn(0, highlightNames.lastIndex)
        var allergenColorChoice = prefs.getInt("allergen_color", 1).coerceIn(0, highlightNames.lastIndex)
        var lineMode = prefs.getBoolean("allergen_line_mode", false)
        var showInlineCodes = prefs.getBoolean("show_inline_allergen_codes", true)
        card {
            text("Kiemelések", 22f)
            val inlineSwitch = Switch(this).apply { text = "Allergénkódok az ételeknél is"; textSize = 16f; setTextColor(ink); minHeight = dp(56); isChecked = showInlineCodes; setOnCheckedChangeListener { _, checked -> showInlineCodes = checked; prefs.edit().putBoolean("show_inline_allergen_codes", checked).apply() } }
            root.addView(inlineSwitch)
            text("Kikapcsolva a kódok csak az étkezés alján láthatók.", 14f)
            fun colorChoice(label: String, initial: Int, change: (Int) -> Unit) {
                var selected = initial
                val control = Button(this).apply { isAllCaps = false; textSize = 16f; minHeight = dp(48); setTextColor(ink) }
                fun update() { control.text = "$label: ${highlightNames[selected]}"; control.backgroundTintList = android.content.res.ColorStateList.valueOf(highlightColor(selected)) }
                update(); control.setOnClickListener {
                    AlertDialog.Builder(this).setTitle(label).setSingleChoiceItems(highlightNames, selected) { dialog, which -> selected = which; change(which); update(); dialog.dismiss() }.setNegativeButton("Mégse", null).show()
                }
                root.addView(control)
            }
            colorChoice("Kulcsszó színe", keywordColorChoice) { keywordColorChoice = it; prefs.edit().putInt("keyword_color", it).apply() }
            colorChoice("Allergén színe", allergenColorChoice) { allergenColorChoice = it; prefs.edit().putInt("allergen_color", it).apply() }
            val lineSwitch = Switch(this).apply { text = "Csak az érintett ételsor jelölése"; textSize = 16f; setTextColor(ink); minHeight = dp(56); isChecked = lineMode; setOnCheckedChangeListener { _, checked -> lineMode = checked; prefs.edit().putBoolean("allergen_line_mode", checked).apply() } }
            root.addView(lineSwitch)
            text("A forrás sortörései nem mindig jelölik egyértelműen az ételek határait. Ellenőrizd a teljes étkezés kódjait is.", 14f)
        }
        val mealChecks = mutableMapOf<String, CheckBox>()
        card {
            text("Megjelenített étkezések", 22f)
            text("Válaszd ki, mely étkezések jelenjenek meg az étlapon. Az emlékeztetők beállításait ez nem módosítja.", 14f)
            for ((category, label) in listOf("breakfast" to "Reggeli", "lunch" to "Ebéd", "dinner" to "Vacsora")) {
                val check = CheckBox(this).apply { text = label; textSize = 17f; setTextColor(ink); minHeight = dp(48); isChecked = prefs.getBoolean("show_$category", true); buttonTintList = android.content.res.ColorStateList.valueOf(green) }
                check.setOnCheckedChangeListener { _, checked -> prefs.edit().putBoolean("show_$category", checked).apply() }; root.addView(check); mealChecks[category] = check
            }
        }
        lateinit var enabled: Switch
        var hour = prefs.getInt("hour", 18); var minute = prefs.getInt("minute", 0)
        card {
            text("Holnapi emlékeztető", 22f)
            enabled = Switch(this).apply { text = "Emlékeztetők"; textSize = 17f; setTextColor(ink); minHeight = dp(56); isChecked = prefs.getBoolean("reminders", false); setOnCheckedChangeListener { _, checked ->
                prefs.edit().putBoolean("reminders", checked).apply(); updateReminders()
                if (checked && Build.VERSION.SDK_INT >= 33 && checkSelfPermission("android.permission.POST_NOTIFICATIONS") != android.content.pm.PackageManager.PERMISSION_GRANTED) requestPermissions(arrayOf("android.permission.POST_NOTIFICATIONS"), 1)
            }; root.addView(this) }
            val timeButton = Button(this).apply { text = "Időpont: %02d:%02d".format(hour, minute); isAllCaps = false; minHeight = dp(48); setTextColor(green); backgroundTintList = android.content.res.ColorStateList.valueOf(surface) }
            timeButton.setOnClickListener { TimePickerDialog(this, { _, h, m -> hour = h; minute = m; prefs.edit().putInt("hour", h).putInt("minute", m).apply(); updateReminders(); timeButton.text = "Időpont: %02d:%02d".format(h, m) }, hour, minute, true).show() }; root.addView(timeButton)
            text(if (getSystemService(NotificationManager::class.java).areNotificationsEnabled()) "Értesítések engedélyezve" else "Az értesítések nincsenek engedélyezve", 14f)
            button("Értesítési engedély beállítása") { startActivity(Intent(android.provider.Settings.ACTION_APP_NOTIFICATION_SETTINGS).putExtra(android.provider.Settings.EXTRA_APP_PACKAGE, packageName)) }
            text("Budapesti idő szerint. A jelzés a mentett holnapi menüt használja; nem mond le étkezést. Az Android késleltetheti. Újraindítás után nyisd meg az appot az újraütemezéshez.", 14f)
        }
        val interval = field("Ellenőrzés megnyitáskor • óránként (1–168)", (prefs.getLong("interval",21600)/3600).toString()).apply { inputType = InputType.TYPE_CLASS_NUMBER }
        watch(interval) { value ->
            val hours = value.toLongOrNull()
            if (hours != null && hours in 1..168) { prefs.edit().putLong("interval", hours * 3600).apply(); interval.error = null }
            else interval.error = "1 és 168 közötti óraszámot adj meg; az előző érték marad érvényben."
        }
        button("Étlap és forrás adatai  ›") {
            sourceDetails()
        }
        button("Kapcsolat és token  ›") { settings(true) }
        text("Szerver: " + (Uri.parse(prefs.getString("endpoint", "https://koliapp.lecso.me")).host ?: "nincs beállítva"), 14f)
        button("Mentett menük törlése") { AlertDialog.Builder(this).setTitle("Törlöd a mentett étlapokat?").setMessage("Offline csak újabb sikeres frissítés után lesznek elérhetők.").setNegativeButton("Mégse", null).setPositiveButton("Törlés") { _, _ -> cache().delete(); prefs.edit().remove("etag").putLong("last_sync",0).apply(); cancelReminders(this); selectedMenu=""; selectedDay=""; status="Mentett étlapok törölve"; render() }.show() }
        root.addView(TextView(this).apply {
            text = "Készítette: Lecs0"; textSize = 14f; setTextColor(green)
            gravity = Gravity.CENTER; setPadding(dp(8), dp(24), dp(8), dp(16))
        })
    }
    private fun sourceDetails() {
        screen = "source"; frame()
        try {
            val data = model()
            if (data == null) text("Még nincs mentett étlap.") else {
                val menus = data.getJSONArray("menus")
                val menu = (0 until menus.length()).map { menus.getJSONObject(it) }.firstOrNull { it.optString("menu_id") == selectedMenu } ?: data.optJSONObject("current_menu")
                if (menu != null) {
                    val source = menu.getJSONObject("entry").optString("source_url")
                    button("Eredeti étlap (PDF) ↗") { if (source.startsWith("https://bajaionkormanyzat.hu/")) try { startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(source))) } catch (_: ActivityNotFoundException) { Toast.makeText(this, "Nincs elérhető böngésző.", Toast.LENGTH_LONG).show() } }
                }
                val missing = data.getJSONArray("unpublished")
                for (i in 0 until missing.length()) text(missing.getJSONObject(i).getString("label") + " • Még nincs közzétéve", 14f)
                text("Forrás frissítve: " + formatTime(data.optString("last_successful_update")), 13f)
            }
        } catch (_: Exception) { text("A forrás adatai most nem érhetők el.", 14f) }
        text("A dátumok az önkormányzat oldaláról származnak. Az allergénkódok jelentése és teljessége nem ellenőrzött. Allergia esetén egyeztess a kollégiummal. A kulcsszójelölés nem allergénfelismerés.", 14f)
        button("Vissza a beállításokhoz") { settings() }
    }
    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        val message = if (getSystemService(NotificationManager::class.java).areNotificationsEnabled()) "Értesítések engedélyezve" else "Az értesítések nincsenek engedélyezve"; Toast.makeText(this, message, Toast.LENGTH_LONG).show(); if (screen == "settings") settings()
    }
    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey("api-token", null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
            init(KeyGenParameterSpec.Builder("api-token", KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        }.generateKey()
    }
    private fun saveToken(token: String) {
        val c = Cipher.getInstance("AES/GCM/NoPadding"); c.init(Cipher.ENCRYPT_MODE,key())
        prefs.edit().putString("token",Base64.encodeToString(c.doFinal(token.toByteArray()),Base64.NO_WRAP)).putString("iv",Base64.encodeToString(c.iv,Base64.NO_WRAP)).apply()
    }
    private fun readToken(): String {
        val encrypted = prefs.getString("token",null) ?: return ""
        val c = Cipher.getInstance("AES/GCM/NoPadding"); c.init(Cipher.DECRYPT_MODE,key(),GCMParameterSpec(128,Base64.decode(prefs.getString("iv", ""),Base64.NO_WRAP)))
        return String(c.doFinal(Base64.decode(encrypted,Base64.NO_WRAP)),Charsets.UTF_8)
    }
    private fun schedule(data: JSONObject) {
        cancelReminders(this)
        if (!prefs.getBoolean("reminders", false)) return
        val at = LocalDate.now(zone).atTime(prefs.getInt("hour",18), prefs.getInt("minute",0)).atZone(zone).toInstant().toEpochMilli()
        if (at <= System.currentTimeMillis()) return
        val plans = data.getJSONArray("reminders")
        val ids = mutableSetOf<String>()
        val alarm = getSystemService(AlarmManager::class.java)
        for (i in 0 until plans.length()) {
            val p = plans.getJSONObject(i); val id = p.getString("id")
            if (prefs.getStringSet("delivered",emptySet())!!.contains(id)) continue
            ids.add(id)
            val intent = Intent(this,ReminderReceiver::class.java).setData(Uri.parse("dormmenu://reminder/$id")).putExtra("id",id).putExtra("message",p.getString("message").replace("Tomorrow's breakfast includes", "A holnapi reggeliben szerepel:").replace("Tomorrow's lunch includes", "A holnapi ebédben szerepel:").replace("Tomorrow's dinner includes", "A holnapi vacsorában szerepel:").replace("Check whether you need to cancel your order.", "Ellenőrizd, szükséges-e lemondanod az étkezést.")).putExtra("notify_date",today())
            val pending = PendingIntent.getBroadcast(this,0,intent,PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            alarm.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP,at,pending)
        }
        prefs.edit().putStringSet("scheduled",ids).apply()
    }
}

fun cancelReminders(context: Context) {
    val prefs = context.getSharedPreferences("settings",Context.MODE_PRIVATE)
    for (id in prefs.getStringSet("scheduled",emptySet())!!) {
        val intent = Intent(context,ReminderReceiver::class.java).setData(Uri.parse("dormmenu://reminder/$id"))
        val p = PendingIntent.getBroadcast(context,0,intent,PendingIntent.FLAG_NO_CREATE or PendingIntent.FLAG_IMMUTABLE)
        if (p != null) { context.getSystemService(AlarmManager::class.java).cancel(p); p.cancel() }
    }
    prefs.edit().remove("scheduled").apply()
}
class ReminderReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context,intent: Intent) {
        val prefs=context.getSharedPreferences("settings",Context.MODE_PRIVATE)
        if (!prefs.getBoolean("reminders",false)) return
        val id=intent.getStringExtra("id") ?: return
        if (intent.getStringExtra("notify_date") != LocalDate.now(ZoneId.of("Europe/Budapest")).toString()) return
        val delivered=prefs.getStringSet("delivered",emptySet())!!.toMutableSet()
        if (delivered.contains(id)) return
        val manager=context.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel("menu","Étkezési emlékeztetők",NotificationManager.IMPORTANCE_DEFAULT))
        val open=PendingIntent.getActivity(context,0,Intent(context,MainActivity::class.java),PendingIntent.FLAG_IMMUTABLE)
        try {
            manager.notify(id.hashCode(),Notification.Builder(context,"menu").setSmallIcon(android.R.drawable.ic_dialog_info).setContentTitle("Ellenőrizd a holnapi rendelésed").setContentText(intent.getStringExtra("message")).setStyle(Notification.BigTextStyle().bigText(intent.getStringExtra("message"))).setContentIntent(open).setAutoCancel(true).build())
            delivered.add(id);prefs.edit().putStringSet("delivered",delivered.sorted().takeLast(100).toSet()).apply()
        } catch (_: SecurityException) { }
    }
}
class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context,intent: Intent) {
        if (intent.action in setOf(Intent.ACTION_BOOT_COMPLETED, Intent.ACTION_TIME_CHANGED, Intent.ACTION_TIMEZONE_CHANGED)) cancelReminders(context)
        /* Reopen app to recompute from cached menus. */
    }
}
