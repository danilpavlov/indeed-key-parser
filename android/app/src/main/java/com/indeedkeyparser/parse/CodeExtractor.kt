package com.indeedkeyparser.parse

private val CODE_REGEX = Regex("^\\d{6,8}$")

private fun isCode(raw: String): Boolean = CODE_REGEX.matches(raw.replace(" ", ""))

fun extractEntry(root: UiNode): Entry? {
    val texts = root.children.mapNotNull { it.text?.trim() }.filter { it.isNotEmpty() }
    val codeText = texts.firstOrNull { isCode(it) }
    if (codeText != null) {
        val label = texts.firstOrNull { !isCode(it) }
        if (label != null) return Entry(label, codeText.replace(" ", ""))
    }
    for (child in root.children) {
        extractEntry(child)?.let { return it }
    }
    return null
}
