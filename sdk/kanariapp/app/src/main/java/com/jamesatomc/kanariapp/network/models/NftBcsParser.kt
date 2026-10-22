package com.jamesatomc.kanariapp.network.models

/**
 * Minimal BCS reader for NFT object payloads the RPC did not pre-parse.
 *
 * The server family has three shapes in the wild; `parsed` is `null` whenever
 * the byte layout behind `data` does not match the RPC's standard parser, so
 * the client decodes locally as a fallback instead of showing a bare type tag:
 *
 * - **james `nft::Nft`** (also the RPC's "universal" shape):
 *   `id: UID, name: String, image_url: Url, description: String,
 *   attribute_keys: vector<String>, attribute_values: vector<String>,
 *   number: String, collection_id: address, creator: address`
 * - **kanari_pet `pet::KanariPet`**: same until attributes, which is a
 *   `VecMap<String, String>` (one length prefix, then flat key/value pairs)
 *   instead of two parallel vectors
 * - **framework `nft::Nft`**: `id: UID, name: String, description: Option<String>,
 *   image_url: Option<Url>, attributes: VecMap<String, String>` and nothing else
 *
 * BCS has no field names, only ordered fields, so every attempt is just a
 * sequential read of the expected layout; the first one that consumes cleanly
 * (and yields a non-blank name) wins.
 */
object NftBcsParser {

    private class Reader(val bytes: ByteArray) {
        var pos = 0

        fun take(n: Int): ByteArray {
            if (pos + n > bytes.size) throw IllegalArgumentException("EOF at $pos need $n")
            val out = bytes.copyOfRange(pos, pos + n)
            pos += n
            return out
        }

        fun uleb(): Int {
            var value = 0L
            var shift = 0
            while (true) {
                val b = take(1)[0].toInt() and 0xFF
                value = value or ((b and 0x7F).toLong() shl shift)
                if (b and 0x80 == 0) break
                shift += 7
                if (shift > 35) throw IllegalArgumentException("uleb128 too long")
            }
            if (value > Int.MAX_VALUE.toLong()) throw IllegalArgumentException("uleb128 overflow")
            return value.toInt()
        }

        fun skip(n: Int) {
            take(n)
        }
    }

    private fun readString(r: Reader): String {
        val len = r.uleb()
        if (len > 1 shl 22) throw IllegalArgumentException("String too long: $len")
        return String(r.take(len), Charsets.UTF_8)
    }

    private fun readOptional(r: Reader): String? {
        return when (r.take(1)[0].toInt()) {
            0 -> null
            1 -> readString(r)
            else -> throw IllegalArgumentException("bad Option tag")
        }
    }

    private fun readVecMap(r: Reader): List<Pair<String, String>> {
        val len = r.uleb()
        val entries = ArrayList<Pair<String, String>>(len)
        repeat(len) { entries.add(readString(r) to readString(r)) }
        return entries
    }

    private fun finished(r: Reader): Boolean = r.pos == r.bytes.size

    /** Parse `data` into the shared [NftParsedData], or `null` if no layout matches. */
    fun parse(data: ByteArray): NftParsedData? = listOf(
        ::parseJamesLike,
        ::parsePetLike,
        ::parseFrameworkNft,
    ).firstNotNullOfOrNull { attempt ->
        try {
            attempt(data)
        } catch (_: Exception) {
            null
        }
    }

    private fun parseJamesLike(data: ByteArray): NftParsedData? {
        val r = Reader(data)
        r.skip(32) // UID
        val name = readString(r)
        val imageUrl = readString(r)
        val description = readString(r)
        val keyCount = r.uleb()
        val keys = List(keyCount) { readString(r) }
        val valueCount = r.uleb()
        val values = List(valueCount) { readString(r) }
        val number = readString(r)
        r.skip(32) // collection_id
        r.skip(32) // creator
        if (!finished(r) || name.isBlank()) return null
        return NftParsedData(
            name = name,
            imageUrl = imageUrl.takeIf { it.isNotBlank() },
            description = description,
            number = number,
            attributes = NftAttributes(keys, values),
        )
    }

    private fun parsePetLike(data: ByteArray): NftParsedData? {
        val r = Reader(data)
        r.skip(32) // UID
        val name = readString(r)
        val imageUrl = readString(r)
        val description = readString(r)
        val attributes = readVecMap(r)
        val number = readString(r)
        r.skip(32) // collection_id
        r.skip(32) // creator
        if (!finished(r) || name.isBlank()) return null
        return NftParsedData(
            name = name,
            imageUrl = imageUrl.takeIf { it.isNotBlank() },
            description = description,
            number = number,
            attributes = NftAttributes(
                attributes.map { it.first },
                attributes.map { it.second },
            ),
        )
    }

    private fun parseFrameworkNft(data: ByteArray): NftParsedData? {
        val r = Reader(data)
        r.skip(32) // UID
        val name = readString(r)
        val description = readOptional(r)
        val imageUrl = readOptional(r)
        val attributes = readVecMap(r)
        if (!finished(r) || name.isBlank()) return null
        return NftParsedData(
            name = name,
            imageUrl = imageUrl?.takeIf { it.isNotBlank() },
            description = description ?: "",
            number = "",
            attributes = NftAttributes(
                attributes.map { it.first },
                attributes.map { it.second },
            ),
        )
    }
}
