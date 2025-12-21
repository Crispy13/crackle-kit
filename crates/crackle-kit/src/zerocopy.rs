#[cfg(test)]
mod tests {
    #[test]
    fn slice_vec_len() {
        let mut v:Vec<u8> = vec![];

        assert_eq!(v.len(), 0);
        assert_eq!(v.as_mut_slice().len(), 0);

        let mut v:Vec<u8> = vec![1,2,3];

        assert_eq!(v.len(), 3);
        assert_eq!(v.as_mut_slice().len(), 3);
        
        let before_cap = v.capacity();
        v.clear();
        assert_eq!(v.len(), 0);
        assert_eq!(v.as_mut_slice().len(), 0);
        assert_eq!(before_cap, v.capacity());
    }
}