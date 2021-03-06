(module
    (import "env" "ext_scratch_size" (func $ext_scratch_size (result i32)))
    (import "env" "ext_scratch_read" (func $ext_scratch_read (param i32 i32 i32)))
    (import "env" "ext_set_storage" (func $ext_set_storage (param i32 i32 i32 i32)))
    (import "env" "ext_get_storage" (func $ext_get_storage (param i32) (result i32)))
    (import "env" "ext_println" (func $ext_println (param i32 i32)))
    (import "env" "cap9_call_with_caps" (func $ext_call (param i32 i32 i64 i32 i32 i32 i32 i32 i32) (result i32)))
    (import "env" "cap9_clist" (func $cap9_clist))
    (import "env" "cap9_clist_downgrade" (func $cap9_clist_downgrade (param i32 i32)))
    (import "env" "ext_address" (func $ext_address))
    (import "env" "ext_balance" (func $ext_balance))
    (import "env" "memory" (memory 1 1))

    ;; store_to needs $mem_length*2 bytes
    (func $to_hex_ascii (param $mem_start i32) (param $mem_length i32) (param $store_to i32) (local $i i32) (local $from_addr i32) (local $to_addr i32) (local $char_dict i32)
        (set_local $i (i32.const 0))
        (set_local $char_dict (i32.const 2000))
        (block
            (loop
                (set_local $from_addr (i32.add (get_local $mem_start) (get_local $i)))
                (set_local $to_addr (i32.add (get_local $store_to) (i32.mul (i32.const 2) (get_local $i))))
                ;; store upper four bits
                (i32.store8 (get_local $to_addr) (i32.load (i32.add (get_local $char_dict) (i32.and (i32.const 0x0f) (i32.shr_u (i32.load (get_local $from_addr)) (i32.const 4))))))
                ;; store lower four bits
                (i32.store8 (i32.add (i32.const 1) (get_local $to_addr)) (i32.load (i32.add (get_local $char_dict) (i32.and (i32.const 0x0f) (i32.load (get_local $from_addr))))))

                (set_local $i (i32.add (get_local $i) (i32.const 1)))
                (br_if 1 (i32.ge_u (get_local $i) (get_local $mem_length)))
                (br 0)
            )
        )
    )

    (func $print_storage_value
        (local $scratch_size_temp i32)
        ;; Get the storage value
        (if
            (call $ext_get_storage
                (i32.const 6000) ;; Pointer to storage key
            )
            (then
                ;; No storage value
                (call $ext_println
                    (i32.const 12100) ;; The data buffer
                    (i32.const 25)   ;; The data buffer's length
                )
                )
            (else
                (set_local $scratch_size_temp (call $ext_scratch_size))
                ;; Read the address from the scratch buffer and store it in memory at
                ;; location 7000.
                (call $ext_scratch_read
                    (i32.const 7000) ;; The pointer where to store the data.
                    (i32.const 0) ;; Offset from the start of the scratch buffer.
                    ;; (i32.const 32) ;; Count of bytes to copy.
                    (get_local $scratch_size_temp)
                )
                (call $to_hex_ascii
                    (i32.const 7000)
                    (get_local $scratch_size_temp)
                    (i32.const 5034)
                )
                (call $ext_println
                    (i32.const 5000) ;; The data buffer
                    (i32.add (i32.const 34) (i32.mul (i32.const 2) (get_local $scratch_size_temp)))   ;; The data buffer's length
                )
            )
        )
    )

    (func (export "call") (local $address_size i32) (local $balance_size i32) (local $scratch_size_temp i32)
        (call $ext_scratch_read
            (i32.const 1500)
            (i32.const 0)
            (call $ext_scratch_size)
        )
        (call $to_hex_ascii
            (i32.const 1500)
            (call $ext_scratch_size)
            (i32.const 8024)
        )
        (call $ext_println
            (i32.const 8000) ;; The data buffer
            (i32.add (i32.const 24) (i32.mul (i32.const 2) (call $ext_scratch_size))) ;; The data buffer's length
        )
        (call $ext_balance)
        (set_local $balance_size (call $ext_scratch_size))
        (call $ext_scratch_read
            (i32.const 4000)
            (i32.const 0)
            (get_local $balance_size)
        )
        (call $to_hex_ascii
            (i32.const 4000)
            (get_local $balance_size)
            (i32.const 11020)
        )
        (call $ext_println
            (i32.const 11000) ;; The data buffer
            (i32.add (i32.const 20) (i32.mul (i32.const 2) (get_local $balance_size))) ;; The data buffer's length
        )

        (call $cap9_clist)
        (set_local $balance_size (call $ext_scratch_size))
        (call $ext_scratch_read
            (i32.const 4000)
            (i32.const 0)
            (get_local $balance_size)
        )
        (call $to_hex_ascii
            (i32.const 4000)
            (get_local $balance_size)
            (i32.const 12417)
        )
        (call $ext_println
            (i32.const 12400) ;; The data buffer
            (i32.add (i32.const 17) (i32.mul (i32.const 2) (get_local $balance_size))) ;; The data buffer's length
        )


        (call $ext_set_storage
            (i32.const 6000) ;; Pointer to the key
            (i32.const 1)    ;; Value is not null
            (i32.const 32)   ;; Pointer to the value
            (i32.const 1)    ;; Length of the value
        )
        (call $print_storage_value)

        ;; Store the address of this contract into the scratch buffer
        (call $ext_address)
        (set_local $address_size (call $ext_scratch_size))
        ;; Read the address from the scratch buffer and store it in memory at
        ;; location 64.
        (call $ext_scratch_read
            (i32.const 64) ;; The pointer where to store the data.
            (i32.const 0) ;; Offset from the start of the scratch buffer.
            ;; (i32.const 32) ;; Count of bytes to copy.
            (get_local $address_size)
        )
        ;; "calling from..." message
        ;; Store the address into the message
        (call $to_hex_ascii
            (i32.const 64)
            (get_local $address_size)
            (i32.const 1025)
        )
        (call $ext_println
            (i32.const 1000) ;; The data buffer
            (i32.add (i32.const 25) (i32.mul (i32.const 2) (get_local $address_size))) ;; The data buffer's length
        )
        ;; "calling to..." message
        (call $to_hex_ascii
            (i32.const 1500)
            (get_local $address_size)
            (i32.const 1223)
        )
        (call $ext_println
            (i32.const 1200) ;; The data buffer
            (i32.add (i32.const 23) (i32.mul (i32.const 2) (get_local $address_size))) ;; The data buffer's length
        )

        (i32.store (i32.const 0) (call $ext_call
            (i32.const 1500) ;; callee_ptr: u32, a pointer to the address of the callee contract. Should be decodable as an `T::AccountId`. Traps otherwise.
            (i32.const 32) ;; callee_len: u32, length of the address buffer.
            (i64.const 0) ;; gas: u64, how much gas to devote to the execution (0 = all).
            ;; IMPORTANT: This was always failing when value wasn't 32 bytes
            (i32.const 0) ;; value_ptr: u32, a pointer to the buffer with value, how much value to send. Should be decodable as a `T::Balance`. Traps otherwise.
            (i32.const 32) ;; value_len: u32, length of the value buffer.
            (i32.const 50) ;; input_data_ptr: u32, a pointer to a buffer to be used as input data to the callee.
            (i32.const 8) ;; no data sent ;; input_data_len: u32, length of the input data buffer.
            (i32.const 13000) ;; cap_data_ptr
            (i32.const 2) ;; cap_data_len
        ))
        (call $to_hex_ascii
            (i32.const 0)
            (i32.const 4)
            (i32.const 12025)
        )
        (call $ext_println
            (i32.const 12000) ;; The data buffer
            (i32.const 29) ;; The data buffer's length
        )
        (if (i32.load (i32.const 0))
            (then
                (call $ext_println
                    (i32.const 12300) ;; The data buffer
                    (i32.const 30) ;; The data buffer's length
                )
            )
            (else
                (call $ext_println
                    (i32.const 12200) ;; The data buffer
                    (i32.const 35) ;; The data buffer's length
                )
            )
        )
        (call $print_storage_value)
    )
    (func (export "deploy")
        (call $cap9_clist_downgrade
            (i32.const 14000) ;; The data buffer
            (i32.const 2) ;; The data buffer's length
        )
    )
    ;; The value we're passing in our call
    (data (i32.const 0) "\00")
    ;; The number of times we will recurse, we need to have that in storage
    ;; somewhere for it to be useful.
    (data (i32.const 32) "\02")
    (data (i32.const 1000) "[CALLER] Calling From: 0x")
    (data (i32.const 1200) "[CALLER] Calling To: 0x")
    (data (i32.const 1500) "\75\42\96\BF\90\25\43\8B\ED\85\16\FB\57\46\7B\A6\1A\58\6C\C1\61\57\2B\13\AA\42\3B\88\C5\51\B6\14")
    ;; byte to hex conversion table
    (data (i32.const 2000) "0123456789ABCDEF")
    (data (i32.const 5000) "[CALLER] Current Storage Value: 0x")
    ;; Some random storage key (32 bytes)
    (data (i32.const 6000) "\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb\aa\bb")
    (data (i32.const 8000) "[CALLER] Called with: 0x")
    (data (i32.const 11000) "[CALLER] Balance: 0x")
    (data (i32.const 12000) "[CALLER] Return Value: 0x")
    (data (i32.const 12100) "[CALLER] No Storage Value")
    (data (i32.const 12200) "[CALLER] Callee exited successfully") ;; 35
    (data (i32.const 12300) "[CALLER] Callee threw an error") ;; 30
    (data (i32.const 12400) "[CALLER] Caps: 0x") ;; 17
    ;; The capabilities which we will pass to the callee.
    (data (i32.const 13000) "\01\00")
    ;; The capabilities which we downgrade this contract to instrinsically
    ;; possess. I.e. these are the capabilities that this contract will possess when it is instantiated.
    (data (i32.const 14000) "\01\00")
)
