package com.indeedkeyparser.settings

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebhookTargetTest {
    @Test
    fun direct_mode_targets_localhost_on_the_given_port() {
        assertEquals(
            "http://localhost:8080/webhook",
            WebhookTarget.resolveUrl(WebhookMode.DIRECT, 8080, "https://ignored.example"),
        )
    }

    @Test
    fun direct_mode_honours_a_custom_port() {
        assertEquals(
            "http://localhost:9000/webhook",
            WebhookTarget.resolveUrl(WebhookMode.DIRECT, 9000, ""),
        )
    }

    @Test
    fun remote_mode_uses_the_configured_url_trimmed() {
        assertEquals(
            "https://vps.example.com/webhook",
            WebhookTarget.resolveUrl(WebhookMode.REMOTE, 8080, "  https://vps.example.com/webhook  "),
        )
    }

    @Test
    fun only_remote_mode_requires_a_network() {
        assertFalse(WebhookTarget.requiresNetwork(WebhookMode.DIRECT))
        assertTrue(WebhookTarget.requiresNetwork(WebhookMode.REMOTE))
    }
}
