package com.indeedkeyparser.parse

data class UiNode(
    val text: String?,
    val children: List<UiNode> = emptyList(),
    val viewId: String? = null,
)

data class Entry(val account: String, val code: String)
