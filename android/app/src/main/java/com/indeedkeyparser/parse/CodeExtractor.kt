package com.indeedkeyparser.parse

private val CODE_REGEX = Regex("^\\d{6,8}$")

// Indeed Key rows carry stable ids: "elN-tOTP" holds the code, "elN-tName" /
// "elN-tAccount" the labels (see docs/superpowers/notes/2026-08-31-spike-accessibility.md).
private val OTP_ID_REGEX = Regex(".*?(el\\d+)-tOTP$")

private fun isCode(raw: String): Boolean = CODE_REGEX.matches(raw.replace(" ", ""))

fun extractEntry(root: UiNode): Entry? = extractById(root) ?: extractByStructure(root)

/** Preferred path: key off the app's per-row resource ids. */
private fun extractById(root: UiNode): Entry? {
    val all = ArrayList<UiNode>()
    flatten(root, all)
    for (node in all) {
        val id = node.viewId ?: continue
        val prefix = OTP_ID_REGEX.matchEntire(id)?.groupValues?.get(1) ?: continue
        val code = node.text?.trim().orEmpty()
        if (!isCode(code)) continue // masked "••• •••" until the user taps
        val label = labelFor(all, prefix) ?: continue
        return Entry(label, code.replace(" ", ""))
    }
    return null
}

private fun labelFor(all: List<UiNode>, prefix: String): String? {
    fun textOf(suffix: String): String? =
        all.firstOrNull { it.viewId?.endsWith("$prefix-$suffix") == true }
            ?.text?.trim()?.takeIf { it.isNotEmpty() }
    return textOf("tName") ?: textOf("tAccount")
}

/** Fallback: pair a code with a nearby non-code label in the same container. */
private fun extractByStructure(root: UiNode): Entry? {
    val texts = root.children.mapNotNull { it.text?.trim() }.filter { it.isNotEmpty() }
    val codeText = texts.firstOrNull { isCode(it) }
    if (codeText != null) {
        val label = texts.firstOrNull { !isCode(it) }
        if (label != null) return Entry(label, codeText.replace(" ", ""))
    }
    for (child in root.children) {
        extractByStructure(child)?.let { return it }
    }
    return null
}

private fun flatten(node: UiNode, acc: MutableList<UiNode>) {
    acc.add(node)
    for (child in node.children) flatten(child, acc)
}
