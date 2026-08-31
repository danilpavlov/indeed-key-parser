package com.indeedkeyparser.parse

data class UiNode(val text: String?, val children: List<UiNode> = emptyList())

data class Entry(val account: String, val code: String)
