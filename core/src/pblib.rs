use prost::{Message, DecodeError};


pub fn decode_many<T>(raw: &[u8]) -> Result<Vec<T>, DecodeError>
    where T: Message + Default
{
    let mut data = Vec::new();
    decode_loop(raw, &mut |item, _, _| {
        data.push(item);
    })?;
    Ok(data)
}

pub fn decode_loop<T>(raw: &[u8], f: &mut impl FnMut(T, usize, usize)) -> Result<(), DecodeError>
    where T: Message + Default
{
    let mut bytes = prost::bytes::BytesMut::from(&raw[..]);
    let total_len = bytes.len();
    loop {
        if bytes.is_empty() { break; }

        let len = prost::decode_length_delimiter(&mut bytes)?;
        let off = total_len - bytes.len();
        let piece = bytes.split_to(len);
        let item = T::decode(piece)?;
        f(item, off, len);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use prost::Message;

    #[derive(Message)]
    struct TestMessage {
        #[prost(int64, tag="1")]
        pub foo: i64,
    }


    #[test]
    fn encode_decode() {
        let m = TestMessage { foo: 69 };
        let buf = m.encode_length_delimited_to_vec();

        let res: Vec<TestMessage> = super::decode_many(&buf).unwrap();

        assert_eq!(res.len(), 1);
        assert_eq!(res[0].foo, 69);
    }
}
