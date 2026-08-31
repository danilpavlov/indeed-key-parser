package com.indeedkeyparser.send

import com.indeedkeyparser.parse.Entry
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class PayloadTest {
    @Test
    fun payload_matches_contract() {
        val json = JSONObject(buildPayload(Entry("Corp", "123456"), "2026-08-31T12:00:00Z"))
        assertEquals("Corp", json.getString("account"))
        assertEquals("123456", json.getString("code"))
        assertEquals("2026-08-31T12:00:00Z", json.getString("timestamp"))
        assertEquals("com.indeedid.key", json.getString("source"))
    }
}
